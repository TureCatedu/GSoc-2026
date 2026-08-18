require_relative '../ext/mylib'

module Scarpe
  # Custom error classes to handle specific errors returned by the Rust backend.
  class RustPanicError < StandardError; end
  class RustIOError < StandardError; end
  class RustNullPointerError < StandardError; end
  class RustInvalidIdError < StandardError; end

  # Entry point for creating and running a Scarpe application.
  # Initializes the application and sets up the TUI environment.
  def self.app(use_alternate = false, title: "Scarpe App", &block)
    App.new(title, use_alternate: use_alternate, &block)
  end

  # Represents an editable text field in the TUI.
  class EditLine
    def initialize(app, id)
      @app = app
      @id = id
    end

    # Retrieves the current text from the Rust backend for this EditLine.
    def text
      @app.get_node_text(@id)
    end

    # Updates the text of this EditLine in the Rust backend.
    def text=(new_text)
      @app.update_node_text(@id, new_text)
    end
  end

  # Represents a multi-line editable text box in the TUI.
  class EditBox
    def initialize(app, id)
      @app = app
      @id = id
    end

    # Retrieves the current text from the Rust backend for this EditBox.
    def text
      @app.get_node_text(@id)
    end

    # Updates the text of this EditBox in the Rust backend.
    def text=(new_text)
      @app.update_node_text(@id, new_text)
    end

  end

  # Represents a checkbox in the TUI.
  class Checkbox
    def initialize(app, id)
      @app = app
      @id = id
    end

    # Checks whether the checkbox is currently checked.
    def checked?
      @app.get_checkbox_state(@id)
    end
  end

  # Represents a text node in the TUI, allowing for dynamic updates to its content.
  class TextNode
    def initialize(app, id)
      @app = app
      @id = id
    end

    def text=(new_text)
      @app.update_node_text(@id, new_text)
    end
  end

  # Main class for managing the Scarpe application.
  # Handles the virtual DOM, rendering, and event handling.
  class App
    # Predefined color codes for styling nodes.
    COLORS = {
      "black" => 0, "red" => 9, "green" => 10, "yellow" => 11,
      "blue" => 12, "magenta" => 13, "cyan" => 14, "white" => 15,
      "gray" => 8, "dark_red" => 1, "dark_green" => 2, "dark_yellow" => 3,
      "dark_blue" => 4, "dark_magenta" => 5, "dark_cyan" => 6, "light_gray" => 7
    }.freeze

    # Predefined text modifiers for styling nodes.
    MODIFIERS = {
      "bold" => 1, "underlined" => 2, "italic" => 3, "reverse" => 4
    }.freeze

    # Node type codes for creating different types of nodes in the virtual DOM.
    NODE_TYPES = {
      root: 0,
      stack: 1,
      flow: 2,
      text: 3,
      edit_line: 4,
      button: 5,
      checkbox: 6,
      border: 7,
      edit_box: 8,
      dock_bottom: 9,
      scroll_area: 10
    }.freeze

    # Initializes the Scarpe application and sets up the TUI context.
    # This includes creating the root node for the virtual DOM and evaluating the UI block.
    def initialize(title, use_alternate: false, &block)
      @title = title
      @should_quit = false
      @node_stack = [] # Stack to manage the hierarchy of nodes in the virtual DOM.
      @callbacks = {} # Hash to store callbacks for interactive elements.

      # Initialize the TUI context by calling the Rust backend.
      @ctx_ptr = ScarpeTuiBackend.scarpe_tui_init(use_alternate)
      if @ctx_ptr.null?
        raise RustPanicError, "Failed to initialize Scarpe-TUI: Rust Core panicked or returned NULL."
      end

      # Create the root node for the virtual DOM.
      @root_node_id = create_tui_node(NODE_TYPES[:root])
      @node_stack.push(@root_node_id)

      # Evaluate the block to build the UI structure.
      instance_eval(&block) if block_given?
      run_loop
    ensure
      # Ensure the TUI context is freed when the application exits.
      shutdown if @ctx_ptr && !@ctx_ptr.null?
    end

    # Creates a stack container in the TUI. A stack arranges its children vertically.
    def stack(&block)
      stack_id = create_tui_node(NODE_TYPES[:stack])
      link_tui_nodes(@node_stack.last, stack_id)

      @node_stack.push(stack_id)
      instance_eval(&block) if block_given?
      @node_stack.pop
    end

    # Creates a flow container in the TUI. A flow arranges its children horizontally.
    def flow(&block)
      flow_id = create_tui_node(NODE_TYPES[:flow])
      link_tui_nodes(@node_stack.last, flow_id)

      @node_stack.push(flow_id)
      instance_eval(&block) if block_given?
      @node_stack.pop
    end

    def append_to(parent_id, &block)

      @node_stack.push(parent_id)
      
      instance_eval(&block) if block_given?
      
      @node_stack.pop
    end

    # Creates a text node in the TUI with optional styling and returns a TextNode object for dynamic updates.
    def para(text, stroke: nil, fill: nil, modifier: nil)
      para_id = create_tui_node(NODE_TYPES[:text], text.to_s)
      link_tui_nodes(@node_stack.last, para_id)
      apply_style(para_id, stroke: stroke, fill: fill, modifier: modifier)
      TextNode.new(self, para_id)
    end

    # Creates an editable text field in the TUI with optional styling.
    def edit_line(initial_text = "", stroke: nil, fill: nil, modifier: nil, &block)
      id = create_tui_node(NODE_TYPES[:edit_line], initial_text.to_s)
      link_tui_nodes(@node_stack.last, id)
      
      apply_style(id, stroke: stroke, fill: fill, modifier: modifier)
      
      if block_given?
        @callbacks[id] = { parent: @node_stack.last, block: block }
      end
      
      EditLine.new(self, id)
    end

    # Creates a multi-line editable text box in the TUI with optional styling. 
    def edit_box(initial_text = "", stroke: nil, fill: nil, modifier: nil, &block)
      id = create_tui_node(NODE_TYPES[:edit_box], initial_text.to_s)
      link_tui_nodes(@node_stack.last, id)
      
      apply_style(id, stroke: stroke, fill: fill, modifier: modifier)
      
      if block_given?
        @callbacks[id] = { parent: @node_stack.last, block: block }
      end
      
      EditBox.new(self, id)
    end

    # Creates a checkbox in the TUI with optional styling and an optional callback.
    def checkbox(*args, stroke: nil, fill: nil, modifier: nil, &block)
      text = args.first.is_a?(String) ? args.first : nil
      
      id = create_tui_node(NODE_TYPES[:checkbox], text)
      link_tui_nodes(@node_stack.last, id)
      
      apply_style(id, stroke: stroke, fill: fill, modifier: modifier)
      
      if block_given?
        # Store the callback block in the @callbacks hash, associating it with the checkbox's node ID.
        @callbacks[id] = { parent: @node_stack.last, block: block } 
      end
      
      Checkbox.new(self, id)
    end

    # Creates a button in the TUI with optional styling and an optional callback.
    def button(text, stroke: nil, fill: nil, modifier: nil, &block)
      button_id = create_tui_node(NODE_TYPES[:button], text.to_s)
      link_tui_nodes(@node_stack.last, button_id)
      
      apply_style(button_id, stroke: stroke, fill: fill, modifier: modifier)
      
      if block_given?
        @callbacks[button_id] = { parent: @node_stack.last, block: block }
      end
    end

    # Creates a border container with optional styling.
    def border(stroke: nil, fill: nil, modifier: nil, &block)
      border_id = create_tui_node(NODE_TYPES[:border])
      link_tui_nodes(@node_stack.last, border_id)
      
      apply_style(border_id, stroke: stroke, fill: fill, modifier: modifier)

      @node_stack.push(border_id)
      instance_eval(&block) if block_given?
      @node_stack.pop
    end

    # Creates a scrollable area in the TUI.
    def dock_bottom(&block)
      dock_id = create_tui_node(NODE_TYPES[:dock_bottom])
      link_tui_nodes(@node_stack.last, dock_id)
      @node_stack.push(dock_id)
      instance_eval(&block) if block_given?
      @node_stack.pop
    end

    # Creates a scrollable area in the TUI with an optional maximum height and styling.
    def scroll_area(max_height: 10, stroke: nil, fill: nil, modifier: nil, &block)
      scroll_id = create_tui_node(NODE_TYPES[:scroll_area], max_height.to_s)
      link_tui_nodes(@node_stack.last, scroll_id)
      apply_style(scroll_id, stroke: stroke, fill: fill, modifier: modifier)
      @node_stack.push(scroll_id)
      instance_eval(&block) if block_given?
      @node_stack.pop
    end

    # Signals the application to quit.
    def quit
      @should_quit = true
    end

    # Retrieves the text of a node from the Rust backend.
    def get_node_text(node_id)
      str_ptr = ScarpeTuiBackend.scarpe_tui_get_text(@ctx_ptr, node_id)
      return "" if str_ptr.null?

      begin
        ruby_string = str_ptr.read_string
        return ruby_string
      ensure
        ScarpeTuiBackend.scarpe_tui_free_string(str_ptr)
      end
    end

    # Retrieves the state of a checkbox node from the Rust backend.
    def get_checkbox_state(node_id)
      status = ScarpeTuiBackend.scarpe_tui_get_checkbox_state(@ctx_ptr, node_id)
      handle_rust_status!(status)
      status == 1
    end

    # Applies styling to a node using the Rust backend.
    def apply_style(node_id, stroke: nil, fill: nil, modifier: nil)
      fg_code = COLORS[stroke.to_s] || -1
      bg_code = COLORS[fill.to_s] || -1
      mod_code = MODIFIERS[modifier.to_s] || -1

      return if fg_code == -1 && bg_code == -1 && mod_code == -1

      status = ScarpeTuiBackend.scarpe_tui_set_style(@ctx_ptr, node_id, fg_code, bg_code, mod_code)
      handle_rust_status!(status)
    end

    # Creates a new node in the virtual DOM by calling the Rust backend.
    def create_tui_node(type_code, text = nil)
      result = ScarpeTuiBackend.scarpe_tui_create_node(@ctx_ptr, type_code, text)
      handle_rust_status!(result)
      result
    end

    # Updates the text of an existing node in the virtual DOM by calling the Rust backend.
    def update_node_text(node_id, new_text)
      status = ScarpeTuiBackend.scarpe_tui_update_text(@ctx_ptr, node_id, new_text.to_s)
      handle_rust_status!(status)
    end

    # Links a child node to a parent node in the virtual DOM.
    def link_tui_nodes(parent_id, child_id)
      status = ScarpeTuiBackend.scarpe_tui_append_child(@ctx_ptr, parent_id, child_id)
      handle_rust_status!(status)
    end

    # Main loop to continuously render the TUI and poll for events until the application is signaled to quit.
    def run_loop
      loop do
        break if @should_quit

        event_code = ScarpeTuiBackend.scarpe_tui_poll_events(@ctx_ptr)

        if event_code == 1 
          quit
        elsif event_code == 2
          handle_click!
        else
          handle_rust_status!(event_code)
        end

        status_code = ScarpeTuiBackend.scarpe_tui_render(@ctx_ptr)
        handle_rust_status!(status_code)

        sleep 0.01
      end
    end

    # Handles status codes returned by the Rust backend, raising appropriate errors for negative codes.
    def handle_rust_status!(code)
      return if code >= 0

      case code
      when -1
        raise RustNullPointerError, "Scarpe-TUI Fatal: Passed a NULL pointer to Rust."
      when -2
        raise RustPanicError, "Scarpe-TUI Fatal: The Rust Core encountered a Panic."
      when -3
        raise RustIOError, "Scarpe-TUI Error: Terminal I/O failure (Crossterm)."
      when -4
        raise RustInvalidIdError, "Scarpe-TUI Error: Tried to use an invalid or non-existent Node ID."
      else
        raise StandardError, "Scarpe-TUI Unknown Error: code #{code}."
      end
    end

    private

    # Handles button click events by retrieving the clicked button ID from Rust
    # and executing the associated callback block if it exists.
    def handle_click!
      clicked_id = ScarpeTuiBackend.scarpe_tui_get_clicked_button(@ctx_ptr)
      return if clicked_id < 0

      callback_data = @callbacks[clicked_id]
      return unless callback_data

      # Push the parent node of the clicked button onto the stack, evaluate the callback block,
      @node_stack.push(callback_data[:parent])
      instance_eval(&callback_data[:block])
      @node_stack.pop
    end

    # Frees the TUI context in the Rust backend to release resources.
    def shutdown
      ScarpeTuiBackend.scarpe_tui_free_context(@ctx_ptr)
      @ctx_ptr = nil
    end
  end
end