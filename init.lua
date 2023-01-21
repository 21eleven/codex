vim.opt.runtimepath = "~/.local/share/codex,/etc/xdg/nvim,/usr/local/share/nvim/site,/usr/share/nvim/site,/usr/share/nvim/runtime,/lib/nvim,/usr/share/nvim/site/after,/usr/local/share/nvim/site/after,~/gits/codex,~/.config/codex"

local execute = vim.api.nvim_command
local fn = vim.fn
local home_dir = vim.loop.os_homedir()

-- local install_path = home_dir..'/.local/share/codex/nvim/site/pack/packer/start/packer.nvim'
local install_path = fn.stdpath('data') .. '/site/pack/packer/start/packer.nvim'

-- print(vim.inspect(vim.opt.rtp:get()))
-- print(vim.inspect(fn.stdpath('data')))
if fn.empty(fn.glob(install_path)) > 0 then
	fn.system({ 'git', 'clone', 'https://github.com/wbthomason/packer.nvim', install_path })
	execute 'packadd packer.nvim'
end


vim.cmd [[packadd packer.nvim]]

opt = vim.opt
g = vim.g

vim.o.autowriteall = true;
vim.o.clipboard = "unnamedplus"
vim.opt.swapfile = false
vim.opt.backup = false
vim.opt.undodir = fn.stdpath('data') .. "/codex_undodir"
vim.opt.undofile = true

vim.opt.isfname:append("@-@")


function map(mode, lhs, rhs, opts)
	local options = { noremap = true }
	if opts then options = vim.tbl_extend('force', options, opts) end
	vim.api.nvim_set_keymap(mode, lhs, rhs, options)
end

Codex = require("codex")

packer = require('packer')
packer.init({ snapshot_path = home_dir .. "/.cache/codex/packer.nvim",
	compile_path = fn.stdpath('data') .. "/packer_compiled.lua" })
packer.reset()
packer_startup = function(use)
	use 'wbthomason/packer.nvim'
	use {
		'nvim-telescope/telescope.nvim',
		requires = { { 'nvim-lua/plenary.nvim' } }
	}
	use 'MunifTanjim/nui.nvim'
	use 'nvim-lualine/lualine.nvim'
	Codex.config.packages(use)
end
packer_startup(packer.use)

vim.api.nvim_create_autocmd('BufWritePost', { command = 'lua Codex.update_word_count()' })
require('lualine').setup { sections = { lualine_c = { "g:word_count", "filename" } } }
require('telescope').load_extension('codex')
vim.cmd [[ autocmd BufEnter *.md hi nodelink ctermfg=cyan guifg=cyan cterm=bold,underline gui=bold ]]
vim.cmd [[ autocmd BufEnter *.md syn region nodelink start=+\[\[+ end=+\]\]+ ]]
vim.cmd [[autocmd VimEnter * lua Codex.start()]]
vim.cmd [[autocmd VimLeave * lua Codex.stop()]]
