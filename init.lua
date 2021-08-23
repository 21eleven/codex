vim.opt.runtimepath= "~/.local/share/codex,/etc/xdg/nvim,/usr/local/share/nvim/site,/usr/share/nvim/site,/usr/share/nvim/runtime,/lib/nvim,/usr/share/nvim/site/after,/usr/local/share/nvim/site/after"

example_func = function(a, b)
	print("A is: ", a)
	print("B is: ", b)
end

local execute = vim.api.nvim_command
local fn = vim.fn

-- local install_path = home_dir..'/.local/share/codex/nvim/site/pack/packer/start/packer.nvim'
local install_path = fn.stdpath('data')..'/site/pack/packer/start/packer.nvim'

print(vim.inspect(vim.opt.rtp:get()))
print(vim.inspect(fn.stdpath('data')))
if fn.empty(fn.glob(install_path)) > 0 then
	fn.system({'git', 'clone', 'https://github.com/wbthomason/packer.nvim', install_path})
	execute 'packadd packer.nvim'
end


vim.cmd [[packadd packer.nvim]]

require('packer').startup(function()
	use 'wbthomason/packer.nvim'
	use {'dracula/vim', as = 'dracula'}
        use { 'ms-jpq/chadtree', run = 'python -m chadtree deps'}
end)

vim.cmd [[colorscheme dracula]]

local opt = vim.opt
local g = vim.g

local function map(mode, lhs, rhs, opts)
  local options = {noremap = true}
  if opts then options = vim.tbl_extend('force', options, opts) end
  vim.api.nvim_set_keymap(mode, lhs, rhs, options)
end

g.mapleader = ' '

map('n', '<leader>e', ":bdelete!", opt)
map('n', '<left>', '0', opt)
map('n', '<right>', '$', opt)
map('n', '<up>', 'kkkkkkk', opt)
map('n', '<down>', 'jjjjjjj', opt)
map('n', '<leader>nh', '<esc>:', opt)
map('n', '<leader>nn', '<esc>/', opt)
map('n', '<leader>w', ':w!<cr>', opt)
map('n', '<leader><leader>w', ':wq!<cr>', opt)
map('n', '<leader>q', ':q<cr>', opt)
-- map('n', '<leader><leader>e', ':q!<cr>', opt)

map('n', '<leader>j', ':BufferLineCyclePrev<CR>', opt)
map('n', '<leader>k', ':BufferLineCycleNext<CR>', opt)
map('n', '<leader>h', '<C-w>h<CR>0', opt)
-- map('n', '<leader>hh', '<C-w>h<CR>0', opt)
map('n', '<leader>l', '<C-w>l<CR>0', opt)
