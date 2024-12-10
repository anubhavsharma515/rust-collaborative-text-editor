# ECE1724 Final Project Report: Rust-Based Markdown Editor with Collaboration Features

---

### Team Information
**Team Members:**
- **[Jesse Na]**
  - **Student Number:** [1005890788]
  - **Preferred Email Address:** [Partner's Email]

- **Anubhav**
  - **Student Number:** [1004541659]
  - **Preferred Email Address:** [anubhav.sharma@mail.utoronto.ca]

---

## Motivation
The motivation for our project stemmed from a noticeable gap in the Rust ecosystem: the lack of a lightweight, terminal-based collaborative markdown editor. While there are excellent standalone tools for editing markdown, very few integrate terminal-based editing with real-time collaboration. Our aim was to combine markdown editing functionality, file management, and collaboration features to deliver a versatile tool for developers and writers.

The project was an exciting challenge to learn Rust and build a performant system, leveraging Rust's concurrency model and ecosystem libraries. It was also fun to explore how collaborative systems can work using WebSockets, all while filling a niche in the Rust ecosystem.

---

## Objectives
The objectives of our project were:
1. **Build a lightweight markdown editor** that provides an intuitive user experience for terminal users.
2. **Integrate file handling features** such as opening, saving, and previewing markdown files.
3. **Enable real-time collaboration**, where multiple users can edit a document simultaneously with user cursors tracked and displayed in real time.
4. Provide a robust yet simple setup process to make the tool accessible to a broad audience.

---

## Features
The final deliverable offers the following features:
1. **Markdown Editing:**
   - Supports writing and editing markdown syntax in a terminal-based interface.
   - Provides a markdown preview mode.

2. **File Management:**
   - Open markdown files from the local system for editing.
   - Save markdown files back to the local system.

3. **Real-Time Collaboration:**
   - Multiple users can edit the same document simultaneously using WebSockets.
   - Live cursor tracking: Each user's cursor is visually distinguished.
   - Conflict resolution ensures seamless integration of edits.

4. **Customization:**
   - Optional themes for the editing interface.
   - Terminal-based interaction for minimal overhead.

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
