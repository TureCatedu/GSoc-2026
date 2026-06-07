require_relative '../ext/mylib'

module Scarpe
  # Define custom error classes to handle specific errors returned by the Rust backend.
  # These errors correspond to specific failure cases in the Rust code.
  class RustPanicError < StandardError; end
  class RustIOError < StandardError; end
  class RustNullPointerError < StandardError; end
  class RustInvalidIdError < StandardError; end

  # Entry point for creating and running a Scarpe application.
  # This method initializes the application and sets up the TUI environment.
  # Parameters:
  # - `use_alternate`: Whether to use the alternate screen buffer (default: false).
  # - `title`: The title of the application (default: "Scarpe App").
  # - `&block`: A block of code defining the application's UI structure.
  def self.app(use_alternate = false, title: "Scarpe App", &block)
    App.new(title, use_alternate: use_alternate, &block)
  end

  class EditLine
    # Represents an editable text field in the TUI.
    def initialize(app, id)
      @app = app
      @id = id
    end

    # Retrieves the current text from the Rust backend for this EditLine.
    def text
      @app.get_node_text(@id)
    end
  end

  class Checkbox
    # Represents a checkbox element in the TUI.
    def initialize(app, id)
      @app = app
      @id = id
    end

    # Retrieves the current state of the checkbox from the Rust backend.
    def checked?
      @app.get_checkbox_state(@id)
    end
  end

  class App
    COLORS = {
      "black" => 0, "red" => 9, "green" => 10, "yellow" => 11,
      "blue" => 12, "magenta" => 13, "cyan" => 14, "white" => 15,
      "gray" => 8, "dark_red" => 1, "dark_green" => 2, "dark_yellow" => 3,
      "dark_blue" => 4, "dark_magenta" => 5, "dark_cyan" => 6, "light_gray" => 7
    }.freeze

    MODIFIERS = {
      "bold" => 1, "underlined" => 2, "italic" => 3, "reverse" => 4
    }.freeze

    # Initializes the Scarpe application and sets up the TUI context.
    # This includes creating the root node for the virtual DOM and evaluating the UI block.
    def initialize(title, use_alternate: false, &block)
      @title = title
      @should_quit = false
      @node_stack = [] # Stack to manage the hierarchy of nodes in the virtual DOM.
      @callbacks = {} # Hash to store callbacks for interactive elements.

      # Initialize the TUI context by calling the Rust backend.
      # This sets up the environment for rendering the application's UI.
      @ctx_ptr = ScarpeTuiBackend.scarpe_tui_init(use_alternate)
      if @ctx_ptr.null?
        raise RustPanicError, "Failed to initialize Scarpe-TUI: Rust Core panicked or returned NULL."
      end

      @root_node_id = create_tui_node(0) # Root node for the virtual DOM.
      @node_stack.push(@root_node_id)

      instance_eval(&block) if block_given?
      run_loop
    ensure
      shutdown if @ctx_ptr && !@ctx_ptr.null?
    end

    # Creates a stack container in the TUI. A stack arranges its children vertically.
    def stack(&block)
      stack_id = create_tui_node(1) # Type 1: Stack
      link_tui_nodes(@node_stack.last, stack_id)

      @node_stack.push(stack_id)
      instance_eval(&block) if block_given?
      @node_stack.pop
    end

    # Creates a flow container in the TUI. A flow arranges its children horizontally.
    def flow(&block)
      flow_id = create_tui_node(2) # Type 2: Flow
      link_tui_nodes(@node_stack.last, flow_id)

      @node_stack.push(flow_id)
      instance_eval(&block) if block_given?
      @node_stack.pop
    end

    # Creates a paragraph of text in the TUI.
    def para(text, stroke: nil, fill: nil, modifier: nil)
      para_id = create_tui_node(3, text.to_s) # Type 3: Text
      link_tui_nodes(@node_stack.last, para_id)
      
      apply_style(para_id, stroke: stroke, fill: fill, modifier: modifier)
    end

    # Signals the application to quit.
    def quit
      @should_quit = true
    end

    # Creates an editable text field in the TUI.
    def edit_line(initial_text = "", stroke: nil, fill: nil, modifier: nil)
      id = create_tui_node(4, initial_text.to_s) # Type 4: EditLine
      link_tui_nodes(@node_stack.last, id)

      apply_style(id, stroke: stroke, fill: fill, modifier: modifier)

      EditLine.new(self, id)
    end

    # Creates a button in the TUI. If a block is provided, it is executed when the button is clicked.
    def button(text, stroke: nil, fill: nil, modifier: nil, &block)
      button_id = create_tui_node(5, text.to_s) # Type 5: Button
      link_tui_nodes(@node_stack.last, button_id)
      
      apply_style(button_id, stroke: stroke, fill: fill, modifier: modifier)

      @callbacks[button_id] = block if block_given?
    end

    def checkbox(*args, stroke: nil, fill: nil, modifier: nil)

      text = args.first.is_a?(String) ? args.first : nil
      
      id = create_tui_node(6, text) # Tipo 6: Checkbox
      link_tui_nodes(@node_stack.last, id)
      
      apply_style(id, stroke: stroke, fill: fill, modifier: modifier)
      @callbacks[id] = block if block_given?
      
      Checkbox.new(self, id)
    end

    # Creates a decorative border (cornice) in the TUI. It can contain nested elements defined in the block.
    def border(stroke: nil, fill: nil, modifier: nil, &block)
      border_id = create_tui_node(7) # Type 7: Border (Cornice Decorativa)
      link_tui_nodes(@node_stack.last, border_id)
      
      apply_style(border_id, stroke: stroke, fill: fill, modifier: modifier)

      # Push the border node onto the stack to allow nested elements to be linked correctly.
      @node_stack.push(border_id)
      instance_eval(&block) if block_given?
      @node_stack.pop
    end

    # Retrieves the text of a node from the Rust backend.
    # Ensures safe memory handling by freeing the allocated string after use.
    def get_node_text(node_id)
      str_ptr = ScarpeTuiBackend.scarpe_tui_get_text(@ctx_ptr, node_id)
      return "" if str_ptr.null?

      begin
        ruby_string = str_ptr.read_string # Clone the native C-string into a Ruby string.
        return ruby_string
      ensure
        # Ensure memory is freed in Rust to prevent leaks.
        ScarpeTuiBackend.scarpe_tui_free_string(str_ptr.address)
      end
    end


    def get_checkbox_state(node_id)
      status = ScarpeTuiBackend.scarpe_tui_get_checkbox_state(@ctx_ptr, node_id)
      handle_rust_status!(status) # Check for errors in the Rust backend call.
      status == 1
    end


    # Helper method to create a new TUI node by calling the Rust backend.
    def create_tui_node(type_code, text = nil)
      # type_code: 0 = Root, 1 = Stack, 2 = Flow, 3 = Text, 4 = EditLine, 5 = Button
      result = ScarpeTuiBackend.scarpe_tui_create_node(@ctx_ptr, type_code, text)
      handle_rust_status!(result)
      result
    end

    # Helper method to link a child node to a parent node in the TUI hierarchy by calling the Rust backend.
    def link_tui_nodes(parent_id, child_id)
      status = ScarpeTuiBackend.scarpe_tui_append_child(@ctx_ptr, parent_id, child_id)
      handle_rust_status!(status)
    end

    def apply_style(node_id, stroke: nil, fill: nil, modifier: nil)
      fg_code = COLORS[stroke.to_s] || -1
      bg_code = COLORS[fill.to_s] || -1
      mod_code = MODIFIERS[modifier.to_s] || -1

      # If all style parameters are invalid, skip the styling call to avoid unnecessary Rust backend interaction.
      return if fg_code == -1 && bg_code == -1 && mod_code == -1

      status = ScarpeTuiBackend.scarpe_tui_set_style(@ctx_ptr, node_id, fg_code, bg_code, mod_code)
      handle_rust_status!(status)
    end

    # Main loop to continuously render the TUI and poll for events until the application is signaled to quit.
    def run_loop
      loop do
        break if @should_quit

        event_code = ScarpeTuiBackend.scarpe_tui_poll_events(@ctx_ptr)

        if event_code == 1 # STATUS_QUIT
          quit
        elsif event_code == 2 # STATUS_CLICKED
          handle_click!
        else
          handle_rust_status!(event_code)
        end

        status_code = ScarpeTuiBackend.scarpe_tui_render(@ctx_ptr)
        handle_rust_status!(status_code)
        
      end
    end

    # Helper method to handle status codes returned by the Rust backend.
    # If the code is negative, it raises an appropriate error based on the specific code.
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

      callback = @callbacks[clicked_id]
      instance_eval(&callback) if callback
    end

    # Frees the TUI context in the Rust backend to release resources.
    def shutdown
      ScarpeTuiBackend.scarpe_tui_free_context(@ctx_ptr)
      @ctx_ptr = nil
    end
  end
end