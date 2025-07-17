-- MPV Subtitle Monitor Script
-- Captures subtitle text and timing information

local utils = require 'mp.utils'
local msg = require 'mp.msg'

-- Configuration
local output_file = "/tmp/mpv-subtitles.json"
local max_entries = 50  -- Keep last 50 subtitles in memory
local subtitle_history = {}
local last_position = 0
local seek_threshold = 5  -- Clear if seeking more than 5 seconds

-- Helper function to write subtitle data to file
local function write_subtitle_data()
    local file = io.open(output_file, "w")
    if file then
        file:write(utils.format_json(subtitle_history))
        file:close()
    else
        msg.error("Failed to write subtitle file: " .. output_file)
    end
end

-- Function to clear subtitle history
local function clear_history(reason)
    subtitle_history = {}
    write_subtitle_data()
    msg.info("Subtitle history cleared: " .. reason)
end

-- Function to add subtitle to history
local function add_subtitle(text, start_time, end_time)
    if text and text ~= "" then
        -- Create subtitle entry
        local entry = {
            text = text,
            start_time = start_time or mp.get_property_number("time-pos", 0),
            end_time = end_time,
            timestamp = os.time()
        }
        
        -- Add to history
        table.insert(subtitle_history, entry)
        
        -- Keep only last max_entries
        if #subtitle_history > max_entries then
            table.remove(subtitle_history, 1)
        end
        
        -- Write to file
        write_subtitle_data()
        
        msg.info("Subtitle captured: " .. text)
    end
end

-- Monitor subtitle text changes
local function on_subtitle_change(name, value)
    if value and value ~= "" then
        -- Get current playback position
        local current_time = mp.get_property_number("time-pos", 0)
        
        -- Add to history
        add_subtitle(value, current_time, nil)
    end
end

-- Monitor seek events
local function on_seek()
    local current_pos = mp.get_property_number("time-pos", 0)
    
    -- Check if this is a significant seek
    if math.abs(current_pos - last_position) > seek_threshold then
        clear_history("seek detected")
    end
    
    last_position = current_pos
end

-- Monitor playback position for seek detection
local function on_time_pos_change(name, value)
    if value then
        local diff = math.abs(value - last_position)
        
        -- If position jumped significantly and we're not just starting playback
        if diff > seek_threshold and last_position > 0 then
            on_seek()
        else
            last_position = value
        end
    end
end

-- Initialize: observe subtitle text property
mp.observe_property("sub-text", "string", on_subtitle_change)

-- Also monitor secondary subtitle track if active
mp.observe_property("secondary-sub-text", "string", function(name, value)
    if value and value ~= "" then
        local current_time = mp.get_property_number("time-pos", 0)
        add_subtitle("[Secondary] " .. value, current_time, nil)
    end
end)

-- Monitor time position for seek detection
mp.observe_property("time-pos", "number", on_time_pos_change)

-- Clear history on file load
mp.register_event("file-loaded", function()
    clear_history("new file loaded")
    last_position = 0
end)

-- Clear history on explicit seek events
mp.register_event("seek", function()
    on_seek()
end)

-- Write empty file on script load to signal we're running
write_subtitle_data()
msg.info("Subtitle monitor started. Writing to: " .. output_file)