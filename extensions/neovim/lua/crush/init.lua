-- crush.nvim — Neovim plugin for Crush Container Runtime

local M = {}

M.config = {
    crush_cmd = 'crush',
    auto_refresh_secs = 5,
    log_height = 20,
    log_width = 120,
}

function M.setup(opts)
    M.config = vim.tbl_deep_extend('force', M.config, opts or {})

    vim.api.nvim_create_user_command('CrushRun', M.run_project, { desc = 'Run project with Crush' })
    vim.api.nvim_create_user_command('CrushPs', M.list_containers, { desc = 'List Crush containers' })
    vim.api.nvim_create_user_command('CrushLogs', function(o) M.show_logs(o.args) end,
        { nargs = 1, desc = 'Show container logs' })

    vim.keymap.set('n', '<leader>cr', ':CrushRun<CR>', { noremap = true, silent = true })
    vim.keymap.set('n', '<leader>cp', ':CrushPs<CR>', { noremap = true, silent = true })

    -- ⚠ FIX: autocmd inside setup() so users who don't call setup() aren't affected
    vim.api.nvim_create_autocmd('BufRead', {
        pattern = 'Crushfile',
        callback = function()
            vim.bo.filetype = 'toml'
        end,
    })
end

function M.run_project()
    local root = vim.fn.getcwd()
    vim.fn.system({ M.config.crush_cmd, 'run', '--detach', root })
    vim.notify('Crush: container started', vim.log.levels.INFO)
end

function M.list_containers()
    local result = vim.fn.system({ M.config.crush_cmd, 'ps', '--format', 'json' })
    local ok, containers = pcall(vim.json.decode, result)
    if not ok then
        vim.notify('Crush: no containers', vim.log.levels.INFO)
        return
    end

    local items = {}
    for _, c in ipairs(containers) do
        table.insert(items, {
            display = string.format('%s  %s  %s', c.Id:sub(1, 12), c.Image or '?', c.Status or '?'),
            id = c.Id,
        })
    end

    vim.ui.select(items, {
        prompt = 'Containers',
        format_item = function(i) return i.display end,
    }, function(choice)
        if choice then M.show_logs(choice.id) end
    end)
end

function M.show_logs(container_id)
    if not container_id or container_id == '' then return end

    local buf = vim.api.nvim_create_buf(false, true)
    local width, height = M.config.log_width, M.config.log_height

    local win = vim.api.nvim_open_win(buf, true, {
        relative = 'editor', width = width, height = height,
        col = math.floor((vim.o.columns - width) / 2),
        row = math.floor((vim.o.lines - height) / 2),
        style = 'minimal', border = 'rounded',
        title = ' Container Logs: ' .. container_id .. ' ',
    })

    vim.bo[buf].modifiable = true

    local job_id = vim.fn.jobstart({
        M.config.crush_cmd, 'logs', '--tail', '50', container_id
    }, {
        on_stdout = function(_, data)
            if data then
                vim.schedule(function()
                    vim.api.nvim_buf_set_lines(buf, -1, -1, false, data)
                    vim.api.nvim_win_set_cursor(win, { vim.api.nvim_buf_line_count(buf), 0 })
                end)
            end
        end,
        on_stderr = function(_, data)
            if data then vim.schedule(function()
                vim.api.nvim_buf_set_lines(buf, -1, -1, false, data)
            end) end
        end,
    })

    vim.keymap.set('n', 'q', function()
        vim.fn.jobstop(job_id); vim.api.nvim_win_close(win, true)
    end, { buffer = buf })
    vim.keymap.set('n', '<Esc>', function()
        vim.fn.jobstop(job_id); vim.api.nvim_win_close(win, true)
    end, { buffer = buf })
end

-- ⚠ Statusline with caching: prevents spawning crush ps on every redraw
local _statusline_cache = ''
local _last_refresh = 0

function M.statusline()
    local now = vim.uv.hrtime() / 1e9
    if now - _last_refresh > M.config.auto_refresh_secs then
        _last_refresh = now
        vim.uv.new_timer():start(0, 0, vim.schedule_wrap(function()
            local ok, containers = pcall(vim.json.decode,
                vim.fn.system({ M.config.crush_cmd, 'ps', '--format', 'json' }))
            if ok then
                local running = 0
                for _, c in ipairs(containers) do
                    if c.State == 'Running' or c.Status == 'Running' then
                        running = running + 1
                    end
                end
                _statusline_cache = running > 0 and ('  ' .. running) or ''
            end
        end))
    end
    return _statusline_cache
end

return M


