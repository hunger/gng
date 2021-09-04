local startup = {}

local inspect = require("inspect")

function startup.init(pkg_definition)
    pkg_defaults = {
        bootstrap = false,

        url = "",
        bug_url = "",

        build_dependencies = {},
        check_dependencies = {},

        -- packets = {},
    }

    PKG_func, err = loadfile(pkg_definition)

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
        f = PKG[v]
        if f == nil then
            f = function() end
        end

        PKG[v] = nil
        _G[v] = f
    end
end

return startup
