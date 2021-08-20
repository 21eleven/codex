vim.opt.runtimepath= "~/gits/codex,/etc/xdg/nvim,/usr/local/share/nvim/site,/usr/share/nvim/site,/usr/share/nvim/runtime,/lib/nvim,/usr/share/nvim/site/after,/usr/local/share/nvim/site/after"

example_func = function(a, b)
	print("A is: ", a)
	print("B is: ", b)
end

local execute = vim.api.nvim_command
local fn = vim.fn

local install_path = fn.stdpath('data')..'/site/pack/packer/start/packer.nvim'

if fn.empty(fn.glob(install_path)) > 0 then
	fn.system({'git', 'clone', 'https://github.com/wbthomason/packer.nvim', install_path})
	execute 'packadd packer.nvim'
end

vim.cmd [[packadd packer.nvim]]

require('packer').startup(function()
	use 'wbthomason/packer.nvim'
	use {'dracula/vim', as = 'dracula'}
end)

vim.cmd [[colorscheme dracula]]

