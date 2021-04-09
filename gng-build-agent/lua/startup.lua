local startup = {}

function startup.init(pkg_definition)
    print("Init called with \"" .. pkg_definition .. "\"...")
    pkg_defaults = {
        bootstrap = false,

        build_dependencies = {},
        check_dependencies = {},

        packets = {},
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

    for _, f in ipairs({ "prepare", "build", "check", "install", "polish", }) do
        local func = PKG[f]
        if func == nil then
            func = function() end
        end
        _G[f] = func
        PKG[f] = nil
    end
end

return startup
