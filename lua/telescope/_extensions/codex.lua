
local M = {}

function M.entry_maker(node)
  return {
    value = node,
    display = node,
    ordinal = node,
  }
end


return require('telescope').register_extension({
  exports = {
    nodes = function()
      local nodes = require('codex').nodes()
      local Picker = require('telescope.pickers')
      local Finder = require('telescope.finders')
      local Sorter = require('telescope.sorters')
      local finder_fn = Finder.new_table({
        results = nodes,
        entry_maker = M.entry_maker,
      })

      local picker = Picker:new({
        prompt_title = 'codex nodex',
        finder = finder_fn,
        sorter = Sorter.get_generic_fuzzy_sorter(),
        previewer = require('telescope.previewers').new_termopen_previewer({
          get_command = function(entry)
            return {'bat', entry.value }
          end,
        }),
      })

      return picker:find()
    end,
  }

})
