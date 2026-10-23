"""Edit the formant table in src/_vtbl.rs.

usage: python tools/formant_editor.py

Pick a phone on the left and edit its values; the vowel chart and envelope follow along, and the A and B dots
on the chart can be dragged to move F1/F2. Save writes the numbers back into src/_vtbl.rs in place, so comments
and layout are kept and the diff shows only the values that changed. Render saves, runs the synthesizer on the
test text and plays the result; every render is kept under ab/editor/, and Play previous plays the one before
it for an A/B check.

Keys: Ctrl+S save, Ctrl+Z undo, Ctrl+Y redo, F5 render, F6 play, F7 play previous.
"""
import cmath
import math
import os
import re
import shutil
import subprocess
import sys
import threading
import time
import tkinter as tk
from tkinter import messagebox, ttk

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TABLE = os.path.join(ROOT, 'src', '_vtbl.rs')
PHONES = os.path.join(ROOT, 'phone_to_id.txt')
RENDERS = os.path.join(ROOT, 'ab', 'editor')
SR = 22050	# synth rate, for the envelope preview

COLS = [
    '%A', '%B', 'RD_A', 'RD_B', 'NZF_A', 'F1_A', 'F2_A', 'F3_A', 'B1_A', 'B2_A', 'B3_A',
    'A2', 'A3', 'A4', 'A5', 'A6', 'A7', 'A8', 'A9', 'A10', 'AB', 'AF', 'AH', 'AV',
    'NZF_B', 'F1_B', 'F2_B', 'F3_B', 'B1_B', 'B2_B', 'B3_B',
]
C = {name: i for i, name in enumerate(COLS)}
NCOL = len(COLS)

CLASSES = [
    ('Silence', [0]),
    ('Vowels', [1, 2, 3, 4, 5, 6, 7, 8, 13, 14, 15, 19, 20, 21, 29, 30, 38, 39]),
    ('Sonorants', [24, 25, 26, 27, 28, 32, 33, 41, 42]),
    ('Stops', [9, 11, 17, 23, 31, 36, 47]),
    ('Fricatives', [10, 12, 16, 18, 22, 34, 35, 37, 40, 43, 44]),
    ('Internal', [45, 46, 48, 49]),
]
VOWELS = set(CLASSES[1][1])
# where the parallel branch's fixed resonators sit (A2 and A3 follow F2 and F3)
PAR_HZ = {'A2': 'F2', 'A3': 'F3', 'A4': '3.75k', 'A5': '4.5k', 'A6': '5.2k', 'A7': '6.5k', 'A8': '7.5k',
          'A9': '8.5k', 'A10': '9.5k'}
BW_FLOOR = 60	# the synth raises cascade B1/B2 below this
K = 1.0		# screen pixels per 96-dpi pixel, set at startup


def px(n):
    return round(n * K)


class Table:
    """The numbers of formant_tbl and where each one sits in the source text."""

    def __init__(self, path):
        self.path = path
        self.load()

    def load(self):
        with open(self.path, encoding='utf-8', newline='') as f:
            self.text = f.read()
        self.mtime = os.path.getmtime(self.path)
        m = re.search(r'formant_tbl\s*:\s*\[u16;\s*(\d+)\]\s*=\s*\[', self.text)
        if not m:
            raise ValueError('formant_tbl not found in ' + self.path)
        size, start = int(m.group(1)), m.end()
        end = self.text.index('];', start)
        self.spans, self.rows, self.labels, self.ready = [], [], [], []
        comments, row = [], []
        for t in re.finditer(r'//[^\n]*|\d+', self.text[start:end]):
            if t.group().startswith('//'):
                comments.append(t.group()[2:].strip())
                continue
            if not row:
                self.ready.append('READY' in comments)
                names = [c for c in comments if c != 'READY']
                self.labels.append(names[-1] if names else '')
                comments = []
            self.spans.append((start + t.start(), start + t.end()))
            row.append(int(t.group()))
            if len(row) == NCOL:
                self.rows.append(row)
                row = []
        if row or len(self.spans) != size:
            raise ValueError(f'formant_tbl has {len(self.spans)} values, expected {size} in rows of {NCOL}')
        self.saved = [r[:] for r in self.rows]

    def changed_on_disk(self):
        return os.path.getmtime(self.path) != self.mtime

    def dirty(self, i=None):
        if i is None:
            return self.rows != self.saved
        return self.rows[i] != self.saved[i]

    def save(self):
        flat = [v for r in self.rows for v in r]
        out, last = [], 0
        for (a, b), v in zip(self.spans, flat):
            out.append(self.text[last:a])
            out.append(str(v))
            last = b
        out.append(self.text[last:])
        with open(self.path, 'w', encoding='utf-8', newline='') as f:
            f.write(''.join(out))
        self.load()


def phone_names():
    names = {}
    try:
        with open(PHONES, encoding='utf-8') as f:
            for line in f:
                m = re.match(r'(\S+)\s+(\d+)', line)
                if m:
                    names[int(m.group(2))] = m.group(1)
    except OSError:
        pass
    return names


def envelope_db(fs, bs, freqs):
    """Magnitude in dB of the cascade F1-F3 resonators, each with unity gain at DC."""
    out = []
    for f in freqs:
        z1 = cmath.exp(-2j * math.pi * f / SR)
        h = 1.0
        for F, B in zip(fs, bs):
            if F <= 0 or B <= 0:
                continue
            r = math.exp(-math.pi * B / SR)
            c = 2 * r * math.cos(2 * math.pi * F / SR)
            h *= (1 - c + r * r) / (1 - c * z1 + r * r * z1 * z1)
        out.append(20 * math.log10(max(abs(h), 1e-6)))
    return out


def play(path):
    if sys.platform == 'win32':
        import winsound
        winsound.PlaySound(path, winsound.SND_FILENAME | winsound.SND_ASYNC)
    elif sys.platform == 'darwin':
        subprocess.Popen(['afplay', path])
    else:
        subprocess.Popen(['aplay', '-q', path])


class Editor(tk.Tk):

    def __init__(self):
        super().__init__()
        global K
        K = self.winfo_fpixels('1i') / 96
        self.title('Formant table editor')
        self.geometry(f'{px(1340)}x{px(820)}')
        self.minsize(px(1100), px(700))
        self.table = Table(TABLE)
        self.names = phone_names()
        self.cur = 1
        self.vars = {}		# column -> StringVar
        self.widgets = {}	# column -> entry widget
        self.same = {}		# B column -> BooleanVar for "same as A"
        self.undo, self.redo = [], []
        self.loading = False
        self.drag = None
        self.renders = []
        self.rendering = False

        style = ttk.Style(self)
        if 'vista' in style.theme_names():
            style.theme_use('vista')
        style.configure('Head.TLabel', font=('Segoe UI', 18, 'bold'))
        style.configure('Sub.TLabel', foreground='#666')
        style.configure('Treeview', rowheight=px(20))
        style.configure('Warn.TLabel', foreground='#a35200')

        self.build_toolbar()
        body = ttk.PanedWindow(self, orient='horizontal')
        body.pack(fill='both', expand=True, padx=6, pady=(0, 6))
        body.add(self.build_list(body), weight=0)
        body.add(self.build_form(body), weight=1)
        body.add(self.build_plots(body), weight=1)
        self.status = ttk.Label(self, anchor='w', padding=(8, 2))
        self.status.pack(fill='x', side='bottom')

        for key, fn in [('<Control-s>', self.save), ('<Control-z>', self.do_undo), ('<Control-y>', self.do_redo),
                        ('<F5>', self.render), ('<F6>', lambda e=None: self.play_render(-1)),
                        ('<F7>', lambda e=None: self.play_render(-2))]:
            self.bind_all(key, lambda e, fn=fn: (fn(), 'break')[1])
        self.protocol('WM_DELETE_WINDOW', self.quit_editor)
        self.select(self.cur)

    # ---------- layout ----------

    def build_toolbar(self):
        bar = ttk.Frame(self, padding=6)
        bar.pack(fill='x')
        ttk.Button(bar, text='Save', command=self.save).pack(side='left')
        ttk.Button(bar, text='Revert', command=self.revert).pack(side='left', padx=(4, 0))
        ttk.Button(bar, text='Undo', command=self.do_undo).pack(side='left', padx=(12, 0))
        ttk.Button(bar, text='Redo', command=self.do_redo).pack(side='left', padx=(4, 0))
        ttk.Separator(bar, orient='vertical').pack(side='left', fill='y', padx=12)
        ttk.Label(bar, text='Test text').pack(side='left')
        self.text_var = tk.StringVar(value='The quick brown fox jumps over the lazy dog.')
        ttk.Entry(bar, textvariable=self.text_var, width=44).pack(side='left', padx=6)
        self.render_btn = ttk.Button(bar, text='Render  F5', command=self.render)
        self.render_btn.pack(side='left')
        ttk.Button(bar, text='Play  F6', command=lambda: self.play_render(-1)).pack(side='left', padx=(4, 0))
        ttk.Button(bar, text='Play previous  F7', command=lambda: self.play_render(-2)).pack(side='left', padx=(4, 0))
        self.render_info = ttk.Label(bar, style='Sub.TLabel')
        self.render_info.pack(side='left', padx=10)

    def build_list(self, parent):
        frame = ttk.Frame(parent, width=px(230))
        self.filter_var = tk.StringVar()
        self.filter_var.trace_add('write', lambda *a: self.fill_list())
        ent = ttk.Entry(frame, textvariable=self.filter_var)
        ent.pack(fill='x', pady=(0, 4))
        self.tree = ttk.Treeview(frame, columns=('id', 'flag'), show='tree headings', selectmode='browse')
        self.tree.heading('#0', text='Phone')
        self.tree.heading('id', text='ID')
        self.tree.heading('flag', text='')
        self.tree.column('#0', width=px(120))
        self.tree.column('id', width=px(36), anchor='e')
        self.tree.column('flag', width=px(70), anchor='w')
        self.tree.tag_configure('dirty', foreground='#0057b8', font=('Segoe UI', 9, 'bold'))
        self.tree.tag_configure('group', font=('Segoe UI', 9, 'bold'))
        sb = ttk.Scrollbar(frame, orient='vertical', command=self.tree.yview)
        self.tree.configure(yscrollcommand=sb.set)
        self.tree.pack(side='left', fill='both', expand=True)
        sb.pack(side='left', fill='y')
        self.tree.bind('<<TreeviewSelect>>', self.on_tree_select)
        self.fill_list()
        return frame

    def fill_list(self):
        q = self.filter_var.get().strip().lower()
        self.tree.delete(*self.tree.get_children())
        for group, ids in CLASSES:
            ids = [i for i in ids if i < len(self.table.rows) and (not q or q in self.phone(i).lower()
                                                                   or q in self.table.labels[i].lower())]
            if not ids:
                continue
            g = self.tree.insert('', 'end', iid='g_' + group, text=group, open=True, tags=('group',))
            for i in ids:
                self.tree.insert(g, 'end', iid=str(i), text=self.phone(i), values=(i, self.flag(i)),
                                 tags=('dirty',) if self.table.dirty(i) else ())
        if self.tree.exists(str(self.cur)):
            self.loading = True
            self.tree.selection_set(str(self.cur))
            self.tree.see(str(self.cur))
            self.loading = False

    def flag(self, i):
        return ('● edited  ' if self.table.dirty(i) else '') + ('' if self.table.ready[i] else 'draft')

    def phone(self, i):
        return self.names.get(i, self.table.labels[i] or str(i))

    def spin(self, parent, col, lo, hi, inc=1, width=7):
        var = tk.StringVar()
        w = ttk.Spinbox(parent, textvariable=var, from_=lo, to=hi, increment=inc, width=width,
                        format='%.2f' if col.startswith('RD') else '')
        var.trace_add('write', lambda *a, col=col: self.on_edit(col))
        self.vars[col], self.widgets[col] = var, w
        return w

    def build_form(self, parent):
        outer = ttk.Frame(parent, padding=(10, 0))
        head = ttk.Frame(outer)
        head.pack(fill='x')
        self.head = ttk.Label(head, style='Head.TLabel')
        self.head.pack(side='left')
        self.sub = ttk.Label(head, style='Sub.TLabel')
        self.sub.pack(side='left', padx=10, pady=(8, 0))
        ttk.Button(head, text='Reset phone', command=self.reset_phone).pack(side='right')
        self.copy_var = tk.StringVar()
        copy = ttk.Combobox(head, textvariable=self.copy_var, width=10, state='readonly',
                            values=[self.phone(i) for i in range(len(self.table.rows))])
        copy.pack(side='right', padx=4)
        copy.bind('<<ComboboxSelected>>', self.copy_from)
        ttk.Label(head, text='Copy from').pack(side='right')

        # timing
        tf = ttk.LabelFrame(outer, text='Transitions  (share of the phone\'s length)', padding=8)
        tf.pack(fill='x', pady=(8, 0))
        ttk.Label(tf, text='%A into the phone').grid(row=0, column=0, sticky='w')
        self.spin(tf, '%A', 0, 100).grid(row=0, column=1, padx=(6, 18))
        ttk.Label(tf, text='%B out of the phone').grid(row=0, column=2, sticky='w')
        self.spin(tf, '%B', 0, 100).grid(row=0, column=3, padx=6)
        self.timeline = tk.Canvas(tf, height=px(26), highlightthickness=0, bg='white')
        self.timeline.grid(row=1, column=0, columnspan=5, sticky='ew', pady=(8, 0))
        tf.columnconfigure(4, weight=1)
        self.timeline.bind('<Configure>', lambda e: self.draw_timeline())

        # formants
        ff = ttk.LabelFrame(outer, text='Cascade formants  (Hz)', padding=8)
        ff.pack(fill='x', pady=(8, 0))
        for c, t in enumerate(['', 'Start (A)', 'End (B)', '']):
            ttk.Label(ff, text=t, style='Sub.TLabel').grid(row=0, column=c, sticky='w', padx=4)
        for r, p in enumerate(['F1', 'F2', 'F3', 'B1', 'B2', 'B3'], start=1):
            ttk.Label(ff, text=p).grid(row=r, column=0, sticky='w', padx=(0, 8))
            hi = 5000 if p[0] == 'F' else 1000
            self.spin(ff, p + '_A', 0, hi, 5 if p[0] == 'F' else 1).grid(row=r, column=1, padx=4, pady=1)
            self.spin(ff, p + '_B', 0, hi, 5 if p[0] == 'F' else 1).grid(row=r, column=2, padx=4, pady=1)
            same = tk.BooleanVar()
            ttk.Checkbutton(ff, text='same as A', variable=same,
                            command=lambda p=p: self.on_same(p + '_B')).grid(row=r, column=3, sticky='w', padx=4)
            self.same[p + '_B'] = same

        # source
        sf = ttk.LabelFrame(outer, text='Voice source and amplitudes  (dB)', padding=8)
        sf.pack(fill='x', pady=(8, 0))
        ttk.Label(sf, text='Rd start').grid(row=0, column=0, sticky='w')
        self.spin(sf, 'RD_A', 0.1, 3.0, 0.01).grid(row=0, column=1, padx=(6, 18))
        ttk.Label(sf, text='Rd end').grid(row=0, column=2, sticky='w')
        self.spin(sf, 'RD_B', 0.1, 3.0, 0.01).grid(row=0, column=3, padx=6)
        for k, (col, tip) in enumerate([('AV', 'voicing'), ('AH', 'aspiration'), ('AF', 'frication'),
                                        ('AB', 'bypass')]):
            r, c = 1 + k // 2, k % 2 * 2
            ttk.Label(sf, text=f'{col}  {tip}').grid(row=r, column=c, sticky='w', pady=(6, 0))
            self.spin(sf, col, 0, 80).grid(row=r, column=c + 1, padx=(6, 18), pady=(6, 0))

        # parallel mixer
        pf = ttk.LabelFrame(outer, text='Parallel branch  A2–A10  (dB, frication and bursts)', padding=8)
        pf.pack(fill='x', pady=(8, 0))
        for c, col in enumerate(['A2', 'A3', 'A4', 'A5', 'A6', 'A7', 'A8', 'A9', 'A10']):
            ttk.Label(pf, text=col).grid(row=0, column=c)
            var = tk.StringVar()
            self.vars[col] = var
            sc = tk.Scale(pf, from_=80, to=0, orient='vertical', length=px(110), showvalue=False, width=px(14),
                          command=lambda v, col=col: self.vars[col].set(str(int(float(v)))))
            sc.grid(row=1, column=c, padx=5)
            e = ttk.Entry(pf, textvariable=var, width=4, justify='center')
            e.grid(row=2, column=c)
            ttk.Label(pf, text=PAR_HZ[col], style='Sub.TLabel').grid(row=3, column=c)
            var.trace_add('write', lambda *a, col=col: self.on_edit(col))
            self.widgets[col] = e
            setattr(self, 'scale_' + col, sc)

        # unused
        nf = ttk.LabelFrame(outer, text='Nasal zero  (not read by the synth)', padding=8)
        nf.pack(fill='x', pady=(8, 0))
        ttk.Label(nf, text='NZF A').grid(row=0, column=0)
        self.spin(nf, 'NZF_A', 0, 5000).grid(row=0, column=1, padx=(6, 18))
        ttk.Label(nf, text='NZF B').grid(row=0, column=2)
        self.spin(nf, 'NZF_B', 0, 5000).grid(row=0, column=3, padx=6)
        ttk.Label(nf, text='nasal coupling comes from the phone\'s features', style='Sub.TLabel').grid(
            row=0, column=4, padx=10)

        self.warn = ttk.Label(outer, style='Warn.TLabel', justify='left', wraplength=px(520))
        self.warn.pack(fill='x', pady=(8, 0))
        return outer

    def build_plots(self, parent):
        frame = ttk.Frame(parent)
        ttk.Label(frame, text='Vowel chart  — drag A (●) or B (○) to move F1/F2', style='Sub.TLabel').pack(anchor='w')
        self.chart = tk.Canvas(frame, bg='white', highlightthickness=1, highlightbackground='#ccc', height=px(380))
        self.chart.pack(fill='both', expand=True)
        self.chart.bind('<Configure>', lambda e: self.draw_chart())
        self.chart.bind('<ButtonPress-1>', self.chart_press)
        self.chart.bind('<B1-Motion>', self.chart_motion)
        self.chart.bind('<ButtonRelease-1>', self.chart_release)
        ttk.Label(frame, text='Cascade envelope  F1–F3  (A solid, B dashed)', style='Sub.TLabel').pack(
            anchor='w', pady=(8, 0))
        self.env = tk.Canvas(frame, bg='white', highlightthickness=1, highlightbackground='#ccc', height=px(200))
        self.env.pack(fill='both', expand=False)
        self.env.bind('<Configure>', lambda e: self.draw_envelope())
        return frame

    # ---------- model <-> widgets ----------

    def get(self, col, i=None):
        return self.table.rows[self.cur if i is None else i][C[col]]

    def shown(self, col):
        v = self.get(col)
        return f'{v / 100:.2f}' if col.startswith('RD') else str(v)

    def select(self, i):
        self.cur = i
        self.loading = True
        for col, var in self.vars.items():
            if col in self.same:
                self.same[col].set(self.get(col) == 0)
                a = col[:-1] + 'A'
                var.set(str(self.get(a)) if self.get(col) == 0 else self.shown(col))
                self.widgets[col].state(['disabled'] if self.get(col) == 0 else ['!disabled'])
            else:
                var.set(self.shown(col))
            if hasattr(self, 'scale_' + col):
                getattr(self, 'scale_' + col).set(self.get(col))
        self.loading = False
        label = self.table.labels[i]
        if self.tree.exists(str(i)) and self.tree.selection() != (str(i),):
            self.loading = True
            self.tree.selection_set(str(i))
            self.tree.see(str(i))
            self.loading = False
        self.head.config(text=self.phone(i).upper())
        self.sub.config(text=f'id {i}  ·  {label}  ·  ' + ('ready' if self.table.ready[i] else 'draft'))
        self.refresh()

    def on_tree_select(self, e):
        if self.loading:
            return
        sel = self.tree.selection()
        if sel and not sel[0].startswith('g_'):
            self.select(int(sel[0]))

    def on_edit(self, col):
        if self.loading:
            return
        s = self.vars[col].get().strip()
        try:
            v = round(float(s) * 100) if col.startswith('RD') else int(s)
        except ValueError:
            return
        if not 0 <= v <= 65535 or (col in ('%A', '%B') and v > 255):
            return
        if col in self.same and self.same[col].get():
            return
        self.set_values([(self.cur, C[col], v)], coalesce=True, from_widget=col)

    def on_same(self, col):
        a = self.get(col[:-1] + 'A')
        if self.same[col].get():
            self.set_values([(self.cur, C[col], 0)])
        else:
            self.set_values([(self.cur, C[col], a)])

    def set_values(self, changes, coalesce=False, from_widget=None, record=True):
        """Apply (row, col, value) changes as one undo step."""
        step = [(r, c, self.table.rows[r][c], v) for r, c, v in changes if self.table.rows[r][c] != v]
        if not step:
            return
        for r, c, old, new in step:
            self.table.rows[r][c] = new
        if record:
            now = time.time()
            last = self.undo[-1] if self.undo else None
            if (coalesce and last and len(step) == 1 and len(last[1]) == 1 and last[1][0][:2] == step[0][:2]
                    and now - last[0] < 1.5):
                r, c, old, _ = last[1][0]
                self.undo[-1] = (now, [(r, c, old, step[0][3])])
            else:
                self.undo.append((now, step))
            self.redo.clear()
        if from_widget and all(r == self.cur for r, *_ in step):
            # keep the edited widget as typed, update the rest
            self.sync_others(from_widget)
            self.refresh()
        else:
            self.select(self.cur)

    def sync_others(self, skip):
        self.loading = True
        for col, var in self.vars.items():
            if col == skip:
                continue
            if col in self.same and self.same[col].get():
                var.set(str(self.get(col[:-1] + 'A')))
            elif col not in self.same or not self.same[col].get():
                if var.get() != self.shown(col):
                    var.set(self.shown(col))
        if hasattr(self, 'scale_' + skip):
            getattr(self, 'scale_' + skip).set(self.get(skip))
        self.loading = False

    def do_undo(self):
        if self.undo:
            _, step = self.undo.pop()
            self.set_values([(r, c, old) for r, c, old, new in step], record=False)
            self.redo.append(step)
            self.goto_step(step)

    def do_redo(self):
        if self.redo:
            step = self.redo.pop()
            self.set_values([(r, c, new) for r, c, old, new in step], record=False)
            self.undo.append((0, step))
            self.goto_step(step)

    def goto_step(self, step):
        if step[0][0] != self.cur:
            self.select(step[0][0])
        self.fill_list()

    def copy_from(self, e):
        name = self.copy_var.get()
        src = next(i for i in range(len(self.table.rows)) if self.phone(i) == name)
        self.copy_var.set('')
        if src == self.cur:
            return
        self.set_values([(self.cur, c, v) for c, v in enumerate(self.table.rows[src])])
        self.status.config(text=f'Copied all values from {name}  (Ctrl+Z to undo)')

    def reset_phone(self):
        self.set_values([(self.cur, c, v) for c, v in enumerate(self.table.saved[self.cur])])

    def refresh(self):
        self.draw_timeline()
        self.draw_chart()
        self.draw_envelope()
        self.warn.config(text='\n'.join('⚠ ' + w for w in self.warnings()))
        if self.tree.exists(str(self.cur)):
            self.tree.item(str(self.cur), values=(self.cur, self.flag(self.cur)),
                           tags=('dirty',) if self.table.dirty(self.cur) else ())
        n = sum(self.table.dirty(i) for i in range(len(self.table.rows)))
        self.title('Formant table editor' + (f'  — {n} phone(s) edited, unsaved' if n else ''))

    def warnings(self):
        g, out = self.get, []
        if g('%A') + g('%B') > 100:
            out.append(f'%A + %B is {g("%A") + g("%B")}: the transitions overlap and there is no steady part.')
        fa = [g(f'F{k}_A') for k in (1, 2, 3)]
        if not fa[0] < fa[1] < fa[2]:
            out.append('Start formants are not in order (F1 < F2 < F3).')
        fb = [g(f'F{k}_B') or g(f'F{k}_A') for k in (1, 2, 3)]
        if not fb[0] < fb[1] < fb[2]:
            out.append('End formants are not in order (F1 < F2 < F3).')
        for k in (1, 2):
            for s in 'AB':
                b = g(f'B{k}_{s}') or (g(f'B{k}_A') if s == 'B' else 0)
                if 0 < b < BW_FLOOR:
                    out.append(f'B{k} {s} is {b} Hz; the synth raises cascade B1/B2 to {BW_FLOOR} Hz.')
        if self.cur in VOWELS and any(g(f'A{k}') != 60 for k in range(2, 11)):
            out.append('Vowels should keep A2–A10 at 60 dB, or neighbouring consonants interpolate toward '
                       'other levels.')
        for s in 'AB':
            rd = g('RD_' + s) / 100
            if not 0.3 <= rd <= 2.7:
                out.append(f'Rd {s} is {rd:.2f}, outside the usual 0.3–2.7.')
        return out

    # ---------- drawing ----------

    def draw_timeline(self):
        cv = self.timeline
        cv.delete('all')
        w = cv.winfo_width() - 2
        if w < 20:
            return
        a, b = self.get('%A') / 100, self.get('%B') / 100
        y0, y1, yc = px(4), px(22), px(13)
        cv.create_rectangle(1, y0, w, y1, fill='#eef3f8', outline='#b8c7d6')
        cv.create_rectangle(1, y0, 1 + w * a, y1, fill='#9cc1e6', outline='')
        cv.create_rectangle(w - w * b, y0, w, y1, fill='#e6b89c', outline='')
        cv.create_text(px(6), yc, text='A', anchor='w', font=('Segoe UI', 8, 'bold'))
        cv.create_text(w - px(6), yc, text='B', anchor='e', font=('Segoe UI', 8, 'bold'))
        mid = 1 - a - b
        cv.create_text(w / 2, yc, text='steady' if mid > 0.08 else ('' if mid >= 0 else 'overlap'),
                       font=('Segoe UI', 8), fill='#555' if mid >= 0 else '#a35200')

    # vowel chart axes: F2 runs right to left, F1 top to bottom
    F2_RANGE = (2700, 500)
    F1_RANGE = (150, 950)

    def to_xy(self, f1, f2):
        w, h, m = self.chart.winfo_width(), self.chart.winfo_height(), px(36)
        x = m + (f2 - self.F2_RANGE[0]) / (self.F2_RANGE[1] - self.F2_RANGE[0]) * (w - 2 * m)
        y = m + (f1 - self.F1_RANGE[0]) / (self.F1_RANGE[1] - self.F1_RANGE[0]) * (h - 2 * m)
        return x, y

    def from_xy(self, x, y):
        w, h, m = self.chart.winfo_width(), self.chart.winfo_height(), px(36)
        f2 = self.F2_RANGE[0] + (x - m) / (w - 2 * m) * (self.F2_RANGE[1] - self.F2_RANGE[0])
        f1 = self.F1_RANGE[0] + (y - m) / (h - 2 * m) * (self.F1_RANGE[1] - self.F1_RANGE[0])
        return round(f1 / 5) * 5, round(f2 / 5) * 5

    def ends(self, i):
        g = lambda col: self.get(col, i)
        a = (g('F1_A'), g('F2_A'))
        b = (g('F1_B') or a[0], g('F2_B') or a[1])
        return a, b

    def draw_chart(self):
        cv = self.chart
        cv.delete('all')
        if cv.winfo_width() < 50:
            return
        for f2 in range(500, 2701, 500):
            x, _ = self.to_xy(self.F1_RANGE[0], f2)
            cv.create_line(x, px(30), x, cv.winfo_height() - px(30), fill='#eee')
            cv.create_text(x, cv.winfo_height() - px(18), text=str(f2), fill='#999', font=('Segoe UI', 8))
        for f1 in range(200, 901, 100):
            _, y = self.to_xy(f1, self.F2_RANGE[0])
            cv.create_line(px(30), y, cv.winfo_width() - px(30), y, fill='#eee')
            cv.create_text(px(16), y, text=str(f1), fill='#999', font=('Segoe UI', 8))
        cv.create_text(cv.winfo_width() / 2, px(12), text='F2 (Hz) ←', fill='#999', font=('Segoe UI', 8))
        for i in sorted(VOWELS) + ([self.cur] if self.cur not in VOWELS else []):
            if i >= len(self.table.rows):
                continue
            (a1, a2), (b1, b2) = self.ends(i)
            ax, ay = self.to_xy(a1, a2)
            bx, by = self.to_xy(b1, b2)
            sel = i == self.cur
            col = '#d0501f' if sel else '#7a8fa6'
            if (a1, a2) != (b1, b2):
                cv.create_line(ax, ay, bx, by, arrow='last', fill=col, width=px(2) if sel else px(1))
            if sel:
                r = px(6)
                cv.create_oval(ax - r, ay - r, ax + r, ay + r, fill=col, outline='white', width=px(2))
                if (a1, a2) != (b1, b2):
                    cv.create_oval(bx - r, by - r, bx + r, by + r, fill='white', outline=col, width=px(2))
            else:
                r = 2.5 * K
                cv.create_oval(ax - r, ay - r, ax + r, ay + r, fill=col, outline='')
            cv.create_text(ax + px(9), ay - px(9), text=self.phone(i), fill=col, anchor='w',
                           font=('Segoe UI', 10, 'bold') if sel else ('Segoe UI', 9))

    def chart_press(self, e):
        (a1, a2), (b1, b2) = self.ends(self.cur)
        ax, ay = self.to_xy(a1, a2)
        bx, by = self.to_xy(b1, b2)
        da, db = math.hypot(e.x - ax, e.y - ay), math.hypot(e.x - bx, e.y - by)
        if min(da, db) > px(12):
            self.drag = None
            return
        # B wins only where it is a real target; a "same as A" B moves with A
        self.drag = 'B' if db < da and self.get('F1_B') and self.get('F2_B') else 'A'
        self.drag_start = [self.table.rows[self.cur][:]]

    def chart_motion(self, e):
        if not self.drag:
            return
        f1, f2 = self.from_xy(e.x, e.y)
        f1, f2 = max(50, f1), max(f1 + 50, f2)
        s = self.drag
        self.set_values([(self.cur, C['F1_' + s], f1), (self.cur, C['F2_' + s], f2)], record=False)
        self.status.config(text=f'{self.phone(self.cur)} {s}: F1 {f1}  F2 {f2}')

    def chart_release(self, e):
        if not self.drag:
            return
        before = self.drag_start[0]
        step = [(self.cur, c, before[c], v) for c, v in enumerate(self.table.rows[self.cur]) if before[c] != v]
        if step:
            self.undo.append((time.time(), step))
            self.redo.clear()
        self.drag = None
        self.fill_list()

    def draw_envelope(self):
        cv = self.env
        cv.delete('all')
        w, h, m, t = cv.winfo_width(), cv.winfo_height(), px(30), px(10)
        if w < 50:
            return
        top, bot, fmax = 30, -60, 5000
        for f in range(0, fmax + 1, 1000):
            x = m + f / fmax * (w - 2 * m)
            cv.create_line(x, t, x, h - px(22), fill='#eee')
            cv.create_text(x, h - t, text=f'{f // 1000}k', fill='#999', font=('Segoe UI', 8))
        for d in range(bot, top + 1, 20):
            y = t + (top - d) / (top - bot) * (h - px(32))
            cv.create_text(px(14), y, text=str(d), fill='#999', font=('Segoe UI', 8))
        freqs = [fmax * k / 240 for k in range(241)]
        g = self.get
        for s, dash in (('A', ()), ('B', (4, 3))):
            fs = [g(f'F{k}_{s}') or g(f'F{k}_A') for k in (1, 2, 3)]
            bs = [g(f'B{k}_{s}') or g(f'B{k}_A') for k in (1, 2, 3)]
            if s == 'B' and fs == [g(f'F{k}_A') for k in (1, 2, 3)] and bs == [g(f'B{k}_A') for k in (1, 2, 3)]:
                continue
            bs = [max(b, BW_FLOOR) if k < 2 else b for k, b in enumerate(bs)]
            db = envelope_db(fs, bs, freqs)
            pts = []
            for f, d in zip(freqs, db):
                d = max(bot, min(top, d))
                pts += [m + f / fmax * (w - 2 * m), t + (top - d) / (top - bot) * (h - px(32))]
            cv.create_line(*pts, fill='#0057b8' if s == 'A' else '#d0501f', width=px(2), dash=dash)

    # ---------- files and rendering ----------

    def check_disk(self):
        if self.table.changed_on_disk():
            return messagebox.askyesno('File changed', 'src/_vtbl.rs changed on disk since it was loaded.\n'
                                       'Overwrite it with the editor\'s values?')
        return True

    def save(self):
        if not self.table.dirty():
            self.status.config(text='Nothing to save.')
            return True
        if not self.check_disk():
            return False
        n = sum(self.table.dirty(i) for i in range(len(self.table.rows)))
        self.table.save()
        self.fill_list()
        self.refresh()
        self.status.config(text=f'Saved {n} phone(s) to src/_vtbl.rs')
        return True

    def revert(self):
        if self.table.dirty() and not messagebox.askyesno('Revert', 'Discard all unsaved edits and reload '
                                                           'src/_vtbl.rs?'):
            return
        self.table.load()
        self.undo.clear()
        self.redo.clear()
        self.fill_list()
        self.select(self.cur)
        self.status.config(text='Reloaded src/_vtbl.rs')

    def render(self):
        if self.rendering:
            return
        if self.table.dirty() and not self.save():
            return
        text = self.text_var.get().strip()
        if not text:
            return
        self.rendering = True
        self.render_btn.state(['disabled'])
        self.status.config(text='Building and rendering…')
        label = f'{len(self.renders) + 1:02d}_{self.phone(self.cur)}'
        threading.Thread(target=self.render_worker, args=(text, label), daemon=True).start()

    def render_worker(self, text, label):
        cmd = ['cargo', 'run', '--release', '--quiet', '--', '--16bit', text]
        try:
            p = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
            ok, log = p.returncode == 0, p.stderr + p.stdout
        except OSError as e:
            ok, log = False, str(e)
        path = None
        if ok:
            os.makedirs(RENDERS, exist_ok=True)
            path = os.path.join(RENDERS, label + '.wav')
            shutil.copyfile(os.path.join(ROOT, 'tts_demo.wav'), path)
        self.after(0, self.render_done, ok, log, path)

    def render_done(self, ok, log, path):
        self.rendering = False
        self.render_btn.state(['!disabled'])
        if not ok:
            self.status.config(text='Render failed')
            self.show_log(log)
            return
        self.renders.append(path)
        self.render_info.config(text=f'{len(self.renders)} render(s) in ab/editor  ·  latest '
                                     f'{os.path.basename(path)}')
        self.status.config(text='Rendered ' + os.path.relpath(path, ROOT))
        play(path)

    def play_render(self, k):
        if len(self.renders) >= -k:
            play(self.renders[k])
            self.status.config(text='Playing ' + os.path.basename(self.renders[k]))
        else:
            self.status.config(text='No render to play yet' if k == -1 else 'Only one render so far')

    def show_log(self, log):
        win = tk.Toplevel(self)
        win.title('Build output')
        t = tk.Text(win, width=110, height=30, font=('Consolas', 9))
        t.insert('1.0', log)
        t.pack(fill='both', expand=True)

    def quit_editor(self):
        if self.table.dirty():
            ans = messagebox.askyesnocancel('Unsaved edits', 'Save edits to src/_vtbl.rs before closing?')
            if ans is None or (ans and not self.save()):
                return
        self.destroy()


if __name__ == '__main__':
    if sys.platform == 'win32':
        import ctypes
        try:
            ctypes.windll.shcore.SetProcessDpiAwareness(1)	# sharp text on scaled displays
        except (AttributeError, OSError):
            pass
    Editor().mainloop()
