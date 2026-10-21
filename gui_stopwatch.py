import os
import tkinter as tk
from time import monotonic
import logging
import threading
import winsound
from tkinter import ttk

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

class StopwatchApp:
    def __init__(self, master):
        self.master = master
        master.title("Bot Tracker")
        master.geometry("450x500")
        master.resizable(True, True)
        master.iconbitmap(os.path.join(os.path.dirname(os.path.abspath(__file__)), "bot_icon.ico"))
        self.start_time = None
        self.elapsed_time = 0.0
        self.is_running = False
        self.interval_duration = 0.0
        self.is_looping = False
        self.loop_after_id = None
        self.tick_after_id = None
        self.loop_period = 0.1
        self.next_deadline = 0.0

        self.time_label = tk.Label(master, font=('Helvetica', 48), text="00:00:00", fg='#007bff', bg='#f0f0f0')
        self.time_label.pack(pady=20, padx=10)
        self.progress = ttk.Progressbar(master, length=250, mode="determinate", maximum=100)
        self.progress.pack(pady=5)
        

        button_frame = tk.Frame(master)
        button_frame.pack(pady=10)

        self.start_button = tk.Button(button_frame, text="🟢 СТАРТ", command=self.start_stopwatch, width=10, height=2, bg='#28a745', fg='white')
        self.start_button.grid(row=0, column=0, padx=10)

        self.stop_button = tk.Button(button_frame, text="🔴 СТОП", command=self.stop_stopwatch, width=10, height=2, bg='#dc3545', fg='white', state=tk.DISABLED)
        self.stop_button.grid(row=0, column=1, padx=10)

        self.reset_button = tk.Button(master, text="🔄 СБРОС", command=self.reset_stopwatch)
        self.reset_button.pack(pady=10)

        self.settings_frame = tk.Frame(master)
        self.settings_frame.pack(pady=5)
        self.countdown_timer_label = tk.Label(self.settings_frame, text="", font=('Helvetica', 18), fg='#ffc107', bg='#f0f0f0', width=45, anchor="center")
        self.countdown_timer_label.pack(pady=5, padx=10)

        # Выбор звука
        sound_frame = tk.Frame(self.settings_frame)
        sound_frame.pack(pady=5)
        tk.Label(sound_frame, text="Звук:", bg='#f0f0f0').pack(side=tk.LEFT)
        self.sound_choice = tk.StringVar(value="Beep 880")
        self.sound_enabled = tk.BooleanVar(value=True)
        tk.Checkbutton(self.settings_frame, text="Звук включен", variable=self.sound_enabled, bg='#f0f0f0').pack(pady=2)
        self.hide_ms = tk.BooleanVar(value=True)
        tk.Checkbutton(self.settings_frame, text="Убрать мс", variable=self.hide_ms, command=self.update_time_label, bg='#f0f0f0').pack(pady=2)
        self.update_time_label()
        tk.OptionMenu(sound_frame, self.sound_choice, "Beep 880", "Beep 600", "Beep 1000").pack(side=tk.LEFT)

        # Отступ перед срабатыванием (сек)
        offset_frame = tk.Frame(self.settings_frame)
        offset_frame.pack(pady=5)
        tk.Label(offset_frame, text="Отступ (сек):", bg='#f0f0f0').pack(side=tk.LEFT)
        self.offset_var = tk.DoubleVar(value=0.0)
        tk.Entry(offset_frame, textvariable=self.offset_var, width=6).pack(side=tk.LEFT)

        # Прозрачность
        alpha_frame = tk.Frame(self.settings_frame)
        alpha_frame.pack(pady=5)
        tk.Label(alpha_frame, text="Прозрачность:", bg='#f0f0f0').pack(side=tk.LEFT)
        self.alpha_scale = tk.Scale(alpha_frame, from_=0.3, to=1.0, resolution=0.05, orient=tk.HORIZONTAL, length=150, command=self.set_alpha)
        self.alpha_scale.set(1.0)
        self.alpha_scale.pack(side=tk.LEFT)
        self.set_alpha(1.0)
        # Цвет обратного отсчёта
        color_frame = tk.Frame(self.settings_frame)
        color_frame.pack(pady=5)
        tk.Label(color_frame, text="Цвет отсчёта:", bg='#f0f0f0').pack(side=tk.LEFT)
        self.countdown_color = tk.StringVar(value='#ffc107')
        tk.OptionMenu(color_frame, self.countdown_color, '#ffc107', '#ff5722', '#28a745', '#007bff', '#e83e8c').pack(side=tk.LEFT)
        # Цвет прогресс-бара
        progress_color_frame = tk.Frame(self.settings_frame)
        progress_color_frame.pack(pady=2)
        tk.Label(progress_color_frame, text="Цвет прогресса:", bg='#f0f0f0').pack(side=tk.LEFT)
        self.progress_color = tk.StringVar(value="blue")
        tk.OptionMenu(progress_color_frame, self.progress_color, "blue", "green", "red", "orange").pack(side=tk.LEFT)
        self.progress_color.trace("w", lambda *args: self.apply_progress_color())
        self.apply_progress_color()
        # Поверх всех окон
        self.topmost_var = tk.BooleanVar(value=False)
        self.reaction_enabled = tk.BooleanVar(value=False)
        tk.Checkbutton(self.settings_frame, text="Реакция светом", variable=self.reaction_enabled, bg='#f0f0f0').pack(pady=2)
        reaction_color_frame = tk.Frame(self.settings_frame)
        reaction_color_frame.pack(pady=2)
        tk.Label(reaction_color_frame, text="Цвет реакции:", bg='#f0f0f0').pack(side=tk.LEFT)
        self.reaction_color = tk.StringVar(value="#007bff")
        tk.OptionMenu(reaction_color_frame, self.reaction_color, "#007bff", "#dc3545", "#28a745", "#ffc107").pack(side=tk.LEFT)
        tk.Checkbutton(self.settings_frame, text="Поверх всех окон", variable=self.topmost_var, command=self.toggle_topmost, bg='#f0f0f0').pack(pady=2)

        master.bind("<Double-Button-1>", self.toggle_settings)
        # Bind double-left to root and label too for reliability
        master.bind("<Button-3>", lambda e: self.toggle_settings())
        self.update_time()

    def toggle_topmost(self):
        self.master.attributes("-topmost", bool(self.topmost_var.get()))

    def toggle_settings(self):
        # Скрываем/показываем только панель настроек, остальной layout не трогаем
        if self.settings_frame.winfo_ismapped():
            self.settings_frame.pack_forget()
        else:
            self.settings_frame.pack(pady=5)
        # Подгоняем окно под содержимое: чёткая форма без пустых полей
        self.master.update_idletasks()
        width = self.master.winfo_reqwidth()
        height = self.master.winfo_reqheight()
        self.master.geometry(f"{width}x{height}")

    def apply_progress_color(self):
        style = ttk.Style()
        style.theme_use("default")
        style.configure("Custom.Horizontal.TProgressbar", troughcolor="#e0e0e0", background=self.progress_color.get())
        self.progress.config(style="Custom.Horizontal.TProgressbar")

    def set_alpha(self, val):
        self.master.attributes("-alpha", float(val))

    def update_time(self):
        if self.is_running:
            self.elapsed_time = monotonic() - self.start_time
            self.update_time_label()
        self.master.after(50, self.update_time)

    def update_time_label(self):
        milliseconds = int((self.elapsed_time * 1000) % 1000)
        seconds = int(self.elapsed_time % 60)
        minutes = int((self.elapsed_time // 60) % 60)
        hours = int(self.elapsed_time // 3600)
        ms_text = f".{milliseconds:03}" if not self.hide_ms.get() else ""
        self.time_label.config(text=f"{hours:02}:{minutes:02}:{seconds:02}{ms_text}")

    def start_stopwatch(self):
        if not self.is_running and not self.is_looping:
            self.is_running = True
            self.start_time = monotonic() - self.elapsed_time
            self.start_button.config(state=tk.DISABLED)
            self.stop_button.config(state=tk.NORMAL)
            logging.info("--- СТАРТ ---")

    def stop_stopwatch(self):
        if self.is_running:
            self.is_running = False
            self.interval_duration = self.elapsed_time
            self.start_button.config(state=tk.NORMAL)
            self.stop_button.config(state=tk.NORMAL)
            logging.info(f"--- СТОП: интервал = {self.interval_duration:.3f} сек ---")
            self.countdown_timer_label.config(text=f"🔁 Повтор каждые {self.interval_duration:.3f} сек")
            self.start_loop()

    def _get_offset(self):
        try:
            return float(self.offset_var.get())
        except (tk.TclError, ValueError):
            return 0.0

    def start_loop(self):
        self.is_looping = True
        self.run_interval_cycle()

    def run_interval_cycle(self):
        if not self.is_looping:
            return
        self.loop_period = max(0.1, self.interval_duration - self._get_offset())
        self.next_deadline = monotonic() + self.loop_period
        self.schedule_beep()
        self.tick_countdown()

    def schedule_beep(self):
        if not self.is_looping:
            return
        delay = self.next_deadline - monotonic()
        self.loop_after_id = self.master.after(max(1, int(delay * 1000)), self.fire_beep)

    def fire_beep(self):
        if not self.is_looping:
            return
        self.loop_period = max(0.1, self.interval_duration - self._get_offset())
        self.next_deadline += self.loop_period
        while self.next_deadline <= monotonic():
            self.next_deadline += self.loop_period
        if self.sound_enabled.get():
            freq = {"Beep 880": 880, "Beep 600": 600, "Beep 1000": 1000}.get(str(self.sound_choice.get()), 880)
            threading.Thread(target=winsound.Beep, args=(freq, 400), daemon=True).start()
        if self.reaction_enabled.get():
            original_bg = self.master.cget("bg")
            self.master.config(bg=self.reaction_color.get())
            self.master.after(300, lambda bg=original_bg: self.master.config(bg=bg))
        fmt = ".0f" if self.hide_ms.get() else ".3f"
        self.countdown_timer_label.config(text=f"Интервал: {self.interval_duration:{fmt}} сек | Отступ: {self._get_offset()}s", fg='#dc3545')
        logging.info(f"--- ПОВТОР + ЗВУК (период {self.loop_period:.3f}с) ---")
        self.schedule_beep()

    def tick_countdown(self):
        if not self.is_looping:
            return
        remaining = self.next_deadline - monotonic()
        if remaining > 0:
            self.countdown_timer_label.config(text=f"⏱️ Обратный отсчёт: {int(remaining)}s", fg=self.countdown_color.get())
            pct = max(0, min(100, int((remaining / self.loop_period) * 100)))
            self.progress.config(value=pct)
        self.tick_after_id = self.master.after(100, self.tick_countdown)

    def reset_stopwatch(self):
        self.is_running = False
        self.is_looping = False
        if self.loop_after_id:
            self.master.after_cancel(self.loop_after_id)
            self.loop_after_id = None
        if self.tick_after_id:
            self.master.after_cancel(self.tick_after_id)
            self.tick_after_id = None
        self.start_time = None
        self.elapsed_time = 0.0
        self.interval_duration = 0.0
        self.update_time_label()
        self.progress.config(value=0)
        self.countdown_timer_label.config(text="")
        self.start_button.config(state=tk.NORMAL)
        self.stop_button.config(state=tk.DISABLED)
        logging.info("--- СБРОС ---")

if __name__ == "__main__":
    root = tk.Tk()
    app = StopwatchApp(root)
    root.mainloop()