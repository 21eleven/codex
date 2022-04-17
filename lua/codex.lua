local M = {}

local actions = require "telescope.actions"
local action_state = require "telescope.actions.state"

local Input = require("nui.input")
local event = require("nui.utils.autocmd").event

-- Would be installed where? .local/share/codex?
local plugin_dir = vim.fn.fnamemodify(vim.api.nvim_get_runtime_file("lua/codex.lua", false)[1], ":h:h")
vim.fn.setenv("CODEX_HOME", plugin_dir)

local binary_path = plugin_dir .. "/target/debug/codex"
if vim.fn.executable(binary_path) == 0 then
    binary_path = plugin_dir .. "/target/release/codex"
end

if vim.fn.getenv("CODEX_DEV") == "true" then
  M.config = require "dev_config"
else
  M.config = require "config"
end

local config = M.config
-- make codex directory if needed
if vim.fn.isdirectory(config.codex_directory) == 0 then
  vim.fn.mkdir(config.codex_directory, 'p')
end
vim.fn.setenv("CODEX_RUNTIME_DIR", config.codex_directory)

local _t = {}
if config.git_remote ~= nil then
    vim.fn.setenv("CODEX_GIT_REMOTE", config.git_remote)
end

function M.start()
    if _t.job_id ~= nil then
        return
    end
    -- :h jobstart has on_stdout option...
    _t.job_id = vim.fn.jobstart({ binary_path }, { cwd = config.codex_directory, rpc = true })
    vim.rpcnotify(_t.job_id, "start", config.git_remote)
end

function M.stop()
    if _t.job_id == nil then
        return
    end
    vim.rpcrequest(_t.job_id, "stop")

    -- vim.rpcnotify(_t.job_id, "stop")
    -- vim.fn.jobstop(_t.job_id)
    _t.job_id = nil
end

function M.get_nodes()
  return vim.rpcrequest(_t.job_id, "nodes")
end

function M.entry_maker(node)
  return {
    -- value = node .. '/meta.toml',
    value = node.id .. '/_.md',
    display = node.display,
    ordinal = node.id,
  }
end

function M.nodes()
  local nodes = M.get_nodes()
  local Picker = require('telescope.pickers')
  local Finder = require('telescope.finders')
  local Sorter = require('telescope.sorters')
  local finder_fn = Finder.new_table({
    results = nodes,
    entry_maker = M.entry_maker
  })

  local picker = Picker:new({
    prompt_title = 'codex nodes',
    finder = finder_fn,
    sorter = Sorter.get_generic_fuzzy_sorter(),
    previewer = require('telescope.previewers').new_termopen_previewer({
      get_command = function(entry)
        return {'/usr/bin/bat', '--style=plain', entry.value }
      end,
    }),
  })

  return picker:find()
end

function M.new_node()
  local nodes = M.get_nodes()
  local Picker = require('telescope.pickers')
  local Finder = require('telescope.finders')
  local Sorter = require('telescope.sorters')
  local Previewer = require('telescope.previewers')
  local finder_fn = Finder.new_table({
    results = nodes,
    entry_maker = M.entry_maker
  })

  local picker = Picker:new({
    prompt_title = 'codex nodes',
    finder = finder_fn,
    sorter = Sorter.get_generic_fuzzy_sorter(),
    -- previewer = Previewer.vim_buffer_cat.new(),
    previewer = require('telescope.previewers').new_termopen_previewer({
      get_command = function(entry)
        return {'/usr/bin/bat', '--style=plain', entry.value }
      end,
    }),
    attach_mappings = function(prompt_bufnr, map)
      actions.select_default:replace(function()
        actions.close(prompt_bufnr)
        local selection = action_state.get_selected_entry()
         local input = Input({
          position = "20%",
          size = {
              width = 50,
              height = 2,
          },
          relative = "editor",
          border = {
            style = "single",
            text = {
                top = "child under: " .. selection.display,
                top_align = "center",
            },
          },
          win_options = {
            winblend = 10,
            winhighlight = "Normal:Normal",
          },
        }, {
          prompt = "> ",
          default_value = "",
          on_close = function()
            print("Input closed!")
          end,
          on_submit = function(value)
            print("New node: " .. value .. ", under: ".. selection.display)
            M["create"](selection.ordinal, value)
          end,
        })

        -- mount/open the component
        input:mount()

        -- unmount component when cursor leaves buffer
        input:on(event.BufLeave, function()
          input:unmount()
        end)
      end)
      return true
    end,
  })

  return picker:find()
end

function leftward_delim_pos(ln, col, delim, opposing_delim)
  local len = string.len(ln)
  if len < 1 then
    return nil
  end
  col = math.min(len, col)
  col = math.max(1, col)
  local delim_offset = string.len(delim)-1
  local delim_start = nil
  for i=0, delim_offset, 1 do
    local a = math.max(col-i, 1)
    local b = col+delim_offset-i
    if string.sub(ln, a, b) == delim then
      delim_start = a
    end
  end
  if delim_start == nil then
    local look_left = string.sub(ln, 1, col-1)
    look_left = string.reverse(look_left);
    local i, j = string.find(look_left, delim, nil, true)
    if j == nil then
      return nil
    end
    -- opposing_delim is within link text then it is invalid
    if string.find(string.sub(look_left, 1, i), opposing_delim, nil, true) ~= nil then
      return nil
    end
    delim_start = col-j
  end
  return delim_start
end

function rightward_delim_pos(ln, col, delim, opposing_delim)
  local len = string.len(ln)
  if col > len then
    return nil
  end
  col = math.min(len, col)
  col = math.max(1, col)
  local delim_offset = string.len(delim)-1
  local delim_end = nil
  for i=0, delim_offset, 1 do
    local a = col-i
    local b = col+delim_offset-i
    if string.sub(ln, a, b) == delim then
      delim_end = b
    end
  end
  if delim_end == nil then
    local look_right = string.sub(ln, col+1)
    local i, j = string.find(look_right, delim, nil, true)
    if j == nil then
      return nil
    end
    -- opposing_delim is within link text then it is invalid
    if string.find(string.sub(look_right, 1, i), opposing_delim, nil, true) ~= nil then
      return nil
    end
    delim_end = col+j
  end
  return delim_end
end

function extract_delimited_text(ln, col, left_delim, right_delim, full)
  local delimited_start = leftward_delim_pos(ln, col, left_delim, right_delim)
  local delimited_end = rightward_delim_pos(ln, col, right_delim, left_delim)
  if delimited_start == nil or delimited_end == nil then
    return nil
  elseif full == nil then
    local left = delimited_start+string.len(left_delim)
    local right = delimited_end-string.len(right_delim)
    return string.sub(ln, left, right)
  else
    return string.sub(ln, delimited_start, delimited_end)
  end
end

function M.find_link_id()
  -- get the column of the current cursor
  -- get a string that represents the current line.
  -- is the character under the cursor [ or ]
  -- of so that is a special case
  -- if not then look leftward of the current column
  -- to find the next occurance of "[[" not the string idx
  -- position of it.
  -- look rightward to find the next occurance
  local ln = vim.api.nvim_get_current_line()
  local col = vim.api.nvim_win_get_cursor({ window = 0 })[2];
  return extract_delimited_text(ln, col, "[[", "]]")
end

function M.todo()
  local ln = vim.api.nvim_get_current_line()
  if string.match(ln, "- %[%]") then
    ln = string.gsub(ln, "- %[%]", "- ✅", 1)
  elseif string.match(ln, "- ✅")  then
    ln = string.gsub(ln, "- ✅", "- %[%]", 1)
  elseif string.match(ln, "- ")  then
    ln = string.gsub(ln, "- ", "- %[%] ", 1)
  elseif string.match(ln, "%a") then
    local idx = string.find(ln, "%a", 1)
    if idx == 1 then
      ln = "- [] " .. ln
    else
      ln = string.sub(ln, 0, idx-1) .. "- [] " .. string.sub(ln, idx)
    end
  else
    ln = ln .. "- [] "
  end
  vim.api.nvim_set_current_line(ln)
end
map('n', '<leader>t', ':lua Codex["todo"]()<CR>', opt)
map('i', '<C-t>', ':lua Codex["todo"]()<CR>', opt)
map('n', '<leader>f', ":lua Codex.nodes() <CR>", opt)
map('n', '<leader>c', ":lua Codex.children() <CR>", opt)
map('n', '<leader>p', ":lua Codex.parent() <CR>", opt)
map('n', '<leader>n', ":lua Codex.new_node() <CR>", opt)

function M.plugin_dir()
    return plugin_dir
end

function M.parent()
  local curr_node = string.gsub(vim.fn.expand("%"), "/_.md", "")
  local parent = vim.rpcrequest(_t.job_id, "parent", curr_node)
  vim.cmd("e " .. parent .. "/_.md")
end

function M.children()
  local curr_node = string.gsub(vim.fn.expand("%"), "/_.md", "")
  local nodes = vim.rpcrequest(_t.job_id, "children", curr_node)
  local Picker = require('telescope.pickers')
  local Finder = require('telescope.finders')
  local Sorter = require('telescope.sorters')
  local Previewer = require('telescope.previewers')
  local finder_fn = Finder.new_table({
    results = nodes,
    entry_maker = M.entry_maker
  })

  local picker = Picker:new({
    prompt_title = 'children',
    finder = finder_fn,
    sorter = Sorter.get_generic_fuzzy_sorter(),
    -- previewer = Previewer.vim_buffer_cat.new(),
    previewer = require('telescope.previewers').new_termopen_previewer({
      get_command = function(entry)
        return {'/usr/bin/bat', '--style=plain', entry.value }
      end,
    }),
  })
  return picker:find()
end

setmetatable(M, {
    __index = function(t, k)
        if _t.job_id == nil then
            return nil
        end
        return function(...)
            vim.rpcnotify(_t.job_id, k, ...)
        end
    end,
})

return M
