require 'ffi'

module ScarpeTuiBackend
  extend FFI::Library
  lib_name = "librust_core.#{FFI::Platform::LIBSUFFIX}"
  lib_path = File.expand_path("../rust_core/target/debug/#{lib_name}", __dir__)
  ffi_lib lib_path

  attach_function :scarpe_tui_init, [], :void
  attach_function :scarpe_tui_shutdown, [], :void
  attach_function :scarpe_tui_render, [], :void
  attach_function :scarpe_tui_poll_event, [], :int
  
  attach_function :scarpe_tui_begin_stack, [], :void
  attach_function :scarpe_tui_end_stack, [], :void
  attach_function :scarpe_tui_begin_flow, [], :void
  attach_function :scarpe_tui_end_flow, [], :void
  
  attach_function :scarpe_tui_add_paragraph, [:string], :void
  attach_function :scarpe_tui_add_button, [:int, :string], :void
  
  attach_function :scarpe_tui_add_input, [:int, :string], :void

  attach_function :scarpe_tui_get_input_text, [:int], :pointer 

  attach_function :scarpe_tui_free_string, [:pointer], :void
end

module Scarpe
  def self.app(title: "Scarpe App", &block)
    App.new(title, &block)
  end

  class InputWidget
    def initialize(id)
      @id = id
    end

    def text
      ptr = ScarpeTuiBackend.scarpe_tui_get_input_text(@id)
      str = ptr.read_string 
      ScarpeTuiBackend.scarpe_tui_free_string(ptr)
      str
    end
  end

  class App
    def initialize(title, &block)
      @widget_id_counter = 0
      @callbacks = {}
      @should_quit = false

      ScarpeTuiBackend.scarpe_tui_init
      instance_eval(&block) if block_given?
      
      loop do
        ScarpeTuiBackend.scarpe_tui_render
        event_code = ScarpeTuiBackend.scarpe_tui_poll_event
        break if event_code == -1 || @should_quit
        
        @callbacks[event_code].call if event_code > 0 && @callbacks[event_code]
      end
      
      ScarpeTuiBackend.scarpe_tui_shutdown
    end

    def quit
      @should_quit = true
    end

    def para(text)
      ScarpeTuiBackend.scarpe_tui_add_paragraph(text.to_s)
    end

    def button(text, &block)
      @widget_id_counter += 1
      @callbacks[@widget_id_counter] = block
      ScarpeTuiBackend.scarpe_tui_add_button(@widget_id_counter, text.to_s)
    end

    def edit_line(initial_text = "")
      @widget_id_counter += 1
      id = @widget_id_counter
      ScarpeTuiBackend.scarpe_tui_add_input(id, initial_text.to_s)
      InputWidget.new(id) 
    end

    def stack(&block)
      ScarpeTuiBackend.scarpe_tui_begin_stack; instance_eval(&block); ScarpeTuiBackend.scarpe_tui_end_stack
    end

    def flow(&block)
      ScarpeTuiBackend.scarpe_tui_begin_flow; instance_eval(&block); ScarpeTuiBackend.scarpe_tui_end_flow
    end
  end
end