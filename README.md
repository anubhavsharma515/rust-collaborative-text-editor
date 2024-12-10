# ECE1724 Final Project Report: Rust-Based Markdown Editor with Collaboration Features

---

### Team Information
**Team Members:**
- **Jesse Na**
  - **Student Number:** 1005890788
  - **Preferred Email Address:** [Partner's Email]

- **Anubhav Sharma**
  - **Student Number:** 1004541659
  - **Preferred Email Address:** anubhav.sharma@mail.utoronto.ca

---

## Video Demo

---

## Motivation
The motivation for our project stemmed from a noticeable gap in the Rust ecosystem: the lack of a lightweight, terminal-based collaborative markdown editor.
While there are excellent standalone tools for editing markdown, very few integrate terminal-based editing with real-time collaboration.
Our aim was to combine markdown editing functionality, file management, and collaboration features to deliver a versatile tool for developers and writers.

The project was an exciting challenge to learn Rust and build a performant system, leveraging Rust's concurrency model and ecosystem libraries.
It was also fun to explore how collaborative systems can work using WebSockets, all while filling a niche in the Rust ecosystem.

---

## Objectives
The objective of this project is to develop a lightweight, low-latency, terminal-based collaborative text editor in Rust. This tool aims to enable multiple users to collaboratively edit and view text files in real time while providing essential functionalities of popular text editors, such as text styling and markdown preview. By operating entirely within the terminal, the tool fills a gap in the Rust ecosystem, offering a Rust-native, CLI-based solution for collaborative editing.

Instead of relying on turn-based edits or simplistic collaboration models, this project tackles the challenges of real-time editing through a lock-based approach. Due to the complexities encountered with the Cola CRDT crate, particularly in processing concurrent edits in the `iced` text-editor interface, our system enforces a locking mechanism. This ensures that users cannot make edits to the same line concurrently but are free to edit other parts of the document. This compromise provides a balance between real-time editing and maintaining document consistency while avoiding potential merge conflicts.

By leveraging Rust’s strengths in performance, safety, and concurrency, this project delivers a robust and responsive terminal application that lays the groundwork for future enhancements, such as syntax-aware editing for collaborative coding.

---

## Features

1. **Custom Client-Side Interface for Text Editing**
   - Implements a terminal-based text editor with basic text-editing operations such as creating, opening, editing, and saving files.
   - Provides button-based and hotkey-enabled text styling options, such as bold, italics, and underline.
   - Includes basic text-analysis tools, such as word, line, paragraph, and character counts.
   - Allows customization through different themes and font families for an enhanced user experience.
   - Supports a markdown preview mode, similar to Obsidian, for `.md` files, enabling live preview of document edits.

2. **Real-Time Collaborative Editing with Lock-Based Concurrency**
   - Enables users to host a file and invite others to collaborate, with options for full or read-only access.
   - Implements lock-based editing to prevent concurrent modifications to the same line of text while allowing edits to other parts of the document.
   - Provides a visual indication of locked lines and active collaborators to enhance usability.
   - Includes session password protection, configurable at runtime or through environment variables.

3. **Networking and Collaboration Features**
   - Uses WebSockets for real-time synchronization between clients.
   - Tracks and displays user cursors in real time, assigning distinct colors to each user.
   - Ensures consistent document state across all participants without the complexity of real-time conflict resolution.

These features prioritize usability, simplicity, and performance, catering to users who prefer terminal environments and seek collaborative tools with essential text editing capabilities. Future iterations of the tool may revisit CRDT-based editing to expand its functionality and flexibility.

---

## User's Guide
### Markdown Editing
1. Launch the editor by running the executable.
2. Use the editor interface to type, modify, or delete markdown text.
3. Toggle preview mode to see rendered markdown.

### File Management
- **Open File:**
  Use the `Open File` button to load a markdown file for editing.
- **Save File:**
  Use the `Save File` button to save changes made during the session.

### Collaboration
1. Start the WebSocket server by running the server executable.
2. Share the session URL with collaborators.
3. Changes made by one user are reflected in real time for all connected users.

---

## Reproducibility Guide
### Prerequisites
- **Rust Toolchain:** Install via [rustup](https://rustup.rs/).
- **Build Environment:** Ubuntu or macOS systems.

### Steps to Set Up and Run
1. **Clone the Repository:**
   ```bash
   git clone <repository-url>
   cd <repository-name>
   ```

2. **Build the Project:**
   ```bash
   cargo build --release
   ```

3. **Start the Markdown Editor**
   ```bash
   ./target/release/editor
   ```
---

## Contributions by Each Team Member

Since this project was
- **Anubhav**:
    - [ ] Implement a **Menu Bar** with options for file picking, theme selection, file saving, and Markdown preview.
    - [ ] Implement a custom **User Cursor Position Marker** to track and display the user’s cursor location within the editor.
    - [ ] Implement a **Status Bar** that supports text-analysis features like character, word and line count

- **Jesse**:
    - [ ] Implement a **Formatting Bar** with basic stylistic controls such as **bold, italic, underline**, **color customization** for text, along with **font size adjustments**.
    - [ ] Set up key-binding shortcuts for common text editing (e.g., **cut, copy, paste, select, delete, and save**) as well as the text-formatting actions (mentioned above).
    - [ ] Implement a custom **Shortcut Palette** widget for displaying supported key-bindings.


Jesse:

	-	Designed the WebSocket server using Axum.
	-	Implemented markdown preview and rendering.
	-	Optimized conflict resolution algorithms for collaborative editing.

---

## Lessons Learned and Concluding Remarks

### Lessons Learned

	•	Rust’s strong type system and concurrency model greatly enhance code safety and performance but require thoughtful design to balance complexity.
	•	Building real-time collaborative systems is challenging, particularly around state synchronization and conflict resolution.
	•	Leveraging community crates (e.g., iced, serde, async-tungstenite) accelerated development significantly.

### Concluding Remarks

This project was both rewarding and educational, providing valuable insights into Rust’s capabilities and its ecosystem. By filling a gap in terminal-based collaborative tools, we hope this project serves as a foundation for others in the community to expand upon.
