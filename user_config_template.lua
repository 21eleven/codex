local M = {}
M.git_remote = "git@githost.net:user/codex.git"
-- g.mapleader = ' '
-- map('i', 'jk', '<esc>', opt)
M.packages = function(use)
  -- use {'dracula/vim', as = 'dracula'}
end

-- vim.cmd [[colorscheme dracula]]
M.codex_directory = vim.loop.os_homedir() .. "/codex"
return M
