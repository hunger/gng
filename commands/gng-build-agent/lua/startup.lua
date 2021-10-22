-- SPDX-License-Identifier: GPL-3.0-or-later
-- Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

local startup = {}

function startup.init(pkg_definition)
    local pkg_defaults = {
        bootstrap = false,

        url = "",
        bug_url = "",

        build_dependencies = {},
        check_dependencies = {},
    }

    local PKG_func, err = loadfile(pkg_definition)

    if PKG_func == nil then
        error("Failed to load \"" .. pkg_definition .. "\" in gng-build-agent: "..err)
    end

    _G.PKG = PKG_func()

    for k, v in pairs(pkg_defaults) do
        if PKG[k] == nil then
            PKG[k] = v
        end
    end

    -- Move functions out of PKG so that we can move it into Rust:
    for _, v in pairs({ "prepare", "build", "check", "install", "polish" }) do
        local f = PKG[v]
        if f == nil then
            f = function() end
        end

        PKG[v] = nil
        _G[v] = f
    end
end

return startup
