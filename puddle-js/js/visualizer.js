import m from "mithril";

const CELL_SIZE = 30;

var Droplet = {
    view: function(vnode) {
        let d = vnode.attrs.droplet;
        let loc = d.location;
        let dim = d.dimensions;
        // nest in a div to avoid confusing css transitions
        return m("div", m(".droplet", {
            key: d.id.id,
            id: `droplet-${d.id.id}`,
            style: {
                left: `${loc.x * CELL_SIZE}px`,
                top: `${loc.y * CELL_SIZE}px`,
                width: `${dim.x * CELL_SIZE}px`,
                height: `${dim.y * CELL_SIZE}px`,
            },
            onclick: function(event, vnode) {
                console.log(event, vnode);
                color = 'blue';
            }
        }))
    }
};

var Module = {
    view: function(vnode) {
        let mod = vnode.attrs.module;
        let loc = mod.location;
        let dim = mod.dimensions;
        return m(".module", {
            style: {
                left: `${loc.x * CELL_SIZE}px`,
                top: `${loc.y * CELL_SIZE}px`,
                width: `${dim.x * CELL_SIZE}px`,
                height: `${dim.y * CELL_SIZE}px`,
            }
        })
    }
};

function Board() {
    var i = 0;

    let data;

    function forward() {
        if (i < data.length - 1) {
            i += 1;
        } else  {
            console.log("can't go forward anymore");
        }
    }

    function backward() {
        if (i > 0) {
            i -= 1;
        } else {
            console.log("can't go backward anymore");
        }
    }

    return {
        view: function (vnode) {
            data = vnode.attrs.data;
            if (!data || data.length == 0) {
                return m("div", "no data yet");
            }
            console.log(data[i]);
            console.log("Droplets: " + data[i].droplets.map(d => d.id.id));
            let droplets = data[i].droplets.map(d => m(Droplet, {droplet: d}));
            let modules = data[i].modules.map(mod => m(Module, {module: mod}));
            return m(
                ".board[tabindex=0]",
                {
                    onkeydown: function(event) {
                        switch (event.key) {
                        case "ArrowRight":
                        case "ArrowDown":
                            forward(data); break;
                        case "ArrowUp":
                        case "ArrowLeft":
                            backward(); break;
                        }
                    }
                },
                [
                    m(".grid", droplets.concat(modules)),
                    m("button", {onclick: backward}, "backward"),
                    m("button", {onclick: forward}, "forward"),
                ]);
        }
    }
}

export { Board as default };
