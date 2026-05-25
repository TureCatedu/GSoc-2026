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

  class App
    # Initializes the Scarpe application and sets up the TUI context.
    # This includes creating the root node for the virtual DOM and evaluating the UI block.
    def initialize(title, use_alternate: false, &block)
      @title = title
      @should_quit = false
      @node_stack = [] # Stack to manage the hierarchy of nodes in the virtual DOM.

      # Initialize the TUI context by calling the Rust backend.
      # This sets up the environment for rendering the application's UI.
      @ctx_ptr = ScarpeTuiBackend.scarpe_tui_init(use_alternate)
      if @ctx_ptr.null?
        raise RustPanicError, "Failed to initialize Scarpe-TUI: Rust Core panicked or returned NULL."
      end

      @root_node_id = create_tui_node(0)
      @node_stack.push(@root_node_id)

      instance_eval(&block) if block_given?
      @node_stack.pop 
      run_loop
    ensure
      shutdown if @ctx_ptr && !@ctx_ptr.null?
    end

    
    def stack(&block)
      stack_id = create_tui_node(1) # Type 1: Stack
      link_tui_nodes(@node_stack.last, stack_id)
      
      @node_stack.push(stack_id)
      instance_eval(&block) if block_given?
      @node_stack.pop
    end

    def flow(&block)
      flow_id = create_tui_node(2) # Type 2: Flow
      link_tui_nodes(@node_stack.last, flow_id)
      
      @node_stack.push(flow_id)
      instance_eval(&block) if block_given?
      @node_stack.pop
    end

    def para(text)
      para_id = create_tui_node(3, text.to_s) # Type 3: Text
      link_tui_nodes(@node_stack.last, para_id)
    end
    def quit
      @should_quit = true
    end

    private

    # Helper method to create a new TUI node by calling the Rust backend.
    def create_tui_node(type_code, text = nil)
      # type_code: 0 = Root, 1 = Stack, 2 = Flow, 3 = Text
      result = ScarpeTuiBackend.scarpe_tui_create_node(@ctx_ptr, type_code, text)
      handle_rust_status!(result)
      result 
    end

    # Helper method to link a child node to a parent node in the TUI hierarchy by calling the Rust backend.
    def link_tui_nodes(parent_id, child_id)
      status = ScarpeTuiBackend.scarpe_tui_append_child(@ctx_ptr, parent_id, child_id)
      handle_rust_status!(status)
    end

    # Main loop to continuously render the TUI and poll for events until the application is signaled to quit.
    def run_loop
      loop do
        break if @should_quit
    
        # 
        status_code = ScarpeTuiBackend.scarpe_tui_render(@ctx_ptr)
        handle_rust_status!(status_code)
    

        event_code = ScarpeTuiBackend.scarpe_tui_poll_events(@ctx_ptr)
        handle_rust_status!(event_code)
        
        if event_code == 1 
          quit
        end
    
        sleep(0.016) # Sleep for ~16ms to target ~60 FPS and reduce CPU usage. Adjust as needed for performance.
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

    def shutdown
      ScarpeTuiBackend.scarpe_tui_free_context(@ctx_ptr)
      @ctx_ptr = nil 
    end
  end
end