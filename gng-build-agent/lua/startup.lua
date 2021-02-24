local startup = {}

function startup.init(pkg_definition)
    print("Init called with \"" .. pkg_definition .. "\"...")
    pkg_defaults = {
        bootstrap = false,

        build_dependencies = {},
        check_dependencies = {},

        prepare = function() end,
        build = function() end,
        check = function() end,
        install = function() end,
        polish = function() end,
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
end

return startup
