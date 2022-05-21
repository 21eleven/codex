vim.opt.runtimepath= "~/.local/share/codex,/etc/xdg/nvim,/usr/local/share/nvim/site,/usr/share/nvim/site,/usr/share/nvim/runtime,/lib/nvim,/usr/share/nvim/site/after,/usr/local/share/nvim/site/after,~/gits/codex,~/.config/codex"

local execute = vim.api.nvim_command
local fn = vim.fn

-- local install_path = home_dir..'/.local/share/codex/nvim/site/pack/packer/start/packer.nvim'
local install_path = fn.stdpath('data')..'/site/pack/packer/start/packer.nvim'

-- print(vim.inspect(vim.opt.rtp:get()))
-- print(vim.inspect(fn.stdpath('data')))
if fn.empty(fn.glob(install_path)) > 0 then
	fn.system({'git', 'clone', 'https://github.com/wbthomason/packer.nvim', install_path})
	execute 'packadd packer.nvim'
end


vim.cmd [[packadd packer.nvim]]

opt = vim.opt
g = vim.g

vim.o.autowriteall = true;
vim.o.clipboard = "unnamedplus"


function map(mode, lhs, rhs, opts)
  local options = {noremap = true}
  if opts then options = vim.tbl_extend('force', options, opts) end
  vim.api.nvim_set_keymap(mode, lhs, rhs, options)
end

Codex = require("codex")

require('packer').startup(function(use)
  use 'wbthomason/packer.nvim'
  use {
    'nvim-telescope/telescope.nvim',
    requires = { {'nvim-lua/plenary.nvim'} }
  }
  use 'MunifTanjim/nui.nvim'
  Codex.config.packages(use)
end)
require('telescope').load_extension('codex')
vim.cmd [[ autocmd BufEnter *.md hi nodelink ctermfg=cyan guifg=cyan cterm=bold,underline gui=bold ]]
vim.cmd [[ autocmd BufEnter *.md syn region nodelink start=+\[\[+ end=+\]\]+ ]]
vim.cmd [[autocmd VimEnter * lua Codex.start()]]
vim.cmd [[autocmd VimLeave * lua Codex.stop()]]
