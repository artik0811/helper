use std::{fmt::Display, fs::File, io::{self, BufRead, BufReader, Write}, process::{Command, Stdio}};


use crossterm::{event::{KeyCode, KeyModifiers}, execute, terminal::{Clear, ClearType}};
use ratatui::{
    buffer::Buffer, 
    layout::{Alignment, Constraint, Layout, Rect}, 
    style::{
        palette::{material::BLACK, tailwind}, Color, Modifier, Style, Styled, Stylize
    }, 
    symbols, 
    text::{Line, Span}, 
    widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph, StatefulWidget, Tabs, Widget}, 
    DefaultTerminal
};
use itertools::Itertools;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};


fn main() -> io::Result<()>{
    let mut terminal = ratatui::init();
    let app = App {exit: AppState::Running, command_list: CommandList::new(), command_running: false};
    let app_result = app.run(&mut terminal);
    ratatui::restore();
    app_result
}

struct App {
    exit: AppState,
    command_list: CommandList,
    command_running: bool
}

#[derive(Default, PartialEq)]
enum AppState {
    #[default]
    Running,
    Quiting,
}

#[derive(Default)]
struct CommandList {
    commands: Vec <CommandItem>,
    state: ListState, 
}

struct CommandItem {
    item: String,
    args: Vec<String>,
}

impl CommandItem {
    fn new (str: String) -> Self {
        let mut parsed_vec = Self::parse_string(str);
        let text = parsed_vec.clone();
        let command = parsed_vec.drain(0..1).next().unwrap();
        let parsed_vec = parsed_vec;    
        Self {
            item: command,
            args: parsed_vec,
        }
    }

    fn parse_string (str: String) -> Vec<String> {
        str.split_whitespace().map(|v| v.to_string()).collect()
    }
}

impl CommandList {
    fn new() -> Self {
        let file = File::open(".config").unwrap();
        let buff = BufReader::new(file);

        let mut config:Vec<CommandItem> = Vec::new();
        for lines in buff.lines() {
            let item = CommandItem::new(lines.unwrap());
            config.push(item);
        }
        

        Self {
            state: ListState::default(),
            commands: config,
        }
    }
}

impl App {
    fn run (mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while self.exit == AppState::Running {
            // if (!self.command_running) {
                terminal.draw(|frame | 
                    frame.render_widget(&mut self, frame.area())
            )?; 
            // }
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key_event) => self.handle_key_event(key_event, terminal)?,
                _=>{}
            }
        }
        Ok(())
    }

    fn handle_key_event (&mut self, key_event: crossterm::event::KeyEvent, terminal: &mut DefaultTerminal) -> io::Result<()> {
        match key_event.code {
            KeyCode::Char('q') | KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.exit = AppState::Quiting;
            }
            KeyCode::Down => self.select_next(),
            KeyCode::Up => self.select_previous(),
            KeyCode::Enter => {
                self.run_selected_command (terminal);
            }
            c => {
                // println!("{}", c);
            }
        }
        Ok(())
    }
//todo: clear display, redraw rendered elements
    fn run_selected_command (&mut self, terminal: &mut DefaultTerminal) {
        self.command_running = true;
        terminal.clear();
        let mut x =terminal.backend();
        let selected = self.command_list.state.selected().unwrap();
        let arg = self.command_list.commands.get(selected).unwrap();
        let mut child = Command::new(arg.item.clone())
        .args(arg.args.clone())
        .stdout(Stdio::piped()) // Перенаправляем stdout в канал
        .spawn() // Запускаем процесс
        .expect("Failed to start process");

    // Получаем доступ к stdout дочернего процесса
        let stdout = child.stdout.take().expect("Failed to open stdout");
        let reader = BufReader::new(stdout);

        // Читаем вывод построчно
        for line in reader.lines() {
            match line {
                Ok(output) => &x.write_fmt(format_args!("{}", output)), // Выводим строку
                Err(_) =>  &x.write_fmt(format_args!("Error")),
            };
        }

        // Дожидаемся завершения процесса
        let exit_status = child.wait().expect("Failed to wait on child process");

        self.command_running = false;
    }

    fn select_next (&mut self)  {
        self.command_list.state.select_next();
    }

    fn select_previous (&mut self)  {
        self.command_list.state.select_previous();
    }

    fn select_by_index (&mut self, index: usize) {
        self.command_list.state.select(Some(index));
    }

    pub fn render_list (&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
        .borders(Borders::all())
        .border_set(symbols::border::THICK);


        let mut items:Vec<String> = Vec::new();

        for line in self.command_list.commands.iter().clone() {
            let mut x= line.item.clone();
            for arg in line.args.clone() {
                x.insert(x.len(), ' ');
                x.insert_str(x.len(), &arg.clone());
            }
            items.push(x.clone());
        }

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
        StatefulWidget::render(list, area, buf, &mut self.command_list.state);
    }

    fn render_bottom_bar (&mut self, area: Rect, buf: &mut Buffer) {
        let keys = [
            ("↑", "Up"),
            ("↓", "Down"),
            ("Enter", "Run"),
            ("Ctrl + Q/C", "Quit"),
        ].to_vec();
        let spans = keys
            .iter()
            .flat_map(|(key, desc)| {
                let key = Span::styled(format!(" {key} "), Style::new().fg(Color::Black).bg(Color::White));
                let desc = Span::styled(format!(" {desc} "), Style::new().fg(Color::White).bg(Color::default()));
                [key, desc]
            })
            .collect_vec();
        Line::from(spans)
            .centered()
            .render(area, buf);
    }

}


impl Widget for &mut App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        if (!self.command_running) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ]);
        let [title_bar, tab, bottom_bar] = vertical.areas(area);
        self.render_list(tab, buf);
        self.render_bottom_bar(bottom_bar, buf);
        }
    }    
}
