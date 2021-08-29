local M = {}

local plugin_dir = vim.fn.fnamemodify(vim.api.nvim_get_runtime_file("lua/codex.lua", false)[1], ":h:h")
vim.fn.setenv("CODEX_HOME", plugin_dir)
-- set codex runtime dir here using some user config setting
local codex_runtime_dir = vim.loop.os_homedir() .. ".local/share/codex"
vim.fn.setenv("CODEX_RUNTIME_DIR", codex_runtime_dir)

local binary_path = plugin_dir .. "/target/debug/codex"
if vim.fn.executable(binary_path) == 0 then
    binary_path = plugin_dir .. "/target/release/codex"
end

local _t = {}

function M.start()
    if _t.job_id ~= nil then
        return
    end
    _t.job_id = vim.fn.jobstart({ binary_path }, { rpc = true })
    vim.rpcnotify(_t.job_id, "start")
end

function M.stop()
    if _t.job_id == nil then
        return
    end
    vim.rpcnotify(_t.job_id, "stop")
    vim.fn.jobstop(_t.job_id)
    _t.job_id = nil
end

function M.plugin_dir()
    return plugin_dir
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
