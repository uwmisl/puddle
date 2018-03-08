let game;
let slider;

let fetch_data = null;

let ready = false; // 'ready' continuous animation checkbox
let running = false; // flag for animation after onComplete
let server_closed = false; // flag that alerts when all data is fetched

let droplets = []; // holds droplets from state-to-state
let prev_json = [];  // holds json over time

let display_frame = 0; // current frame in visualizer
let selected_frame = 0; // goal frame per user
let max_frame = Number.MAX_VALUE; // frame when server is closed

const CELL_SIZE = 50;
const TWEEN_TIME = 200; // in millisec

/**
 * Loads all of the necessary Phaser stuff and initializes
 * the step function.
 */
window.onload = function() {
    game = new Phaser.Game(
        window.innerWidth * window.devicePixelRatio,
        window.innerHeight * window.devicePixelRatio / 2,
        Phaser.CANVAS,
        'game');

    var step = function(game) {};

    step.prototype = {
        // load assets here if we wind up with any
        preload: function() {},

        create: function() {

            game.stage.backgroundColor = "#ffffff";
            game.physics.startSystem(Phaser.Physics.ARCADE);
            slider = document.getElementById("slider");

            fetch_data();

            document.getElementById("back").onclick = backward;

            document.getElementById("step").onclick  = forward;

            document.getElementById("ready").onclick = function() {
                if (this.checked) {
                    run_animation();
                }
                ready = this.checked;
            }

            document.getElementById("slider").oninput = function() {
                selected_frame = slider.value;
                update_frame();
            }

            document.addEventListener('keypress', (event) => {
                const keyName = event.key;
                if (keyName == 'l') {
                    forward();
                } else if (keyName == 'j') {
                    backward();
                }
            });

            game.stage.disableVisibilityChange = true;
        },

        update: function() {},
    };

    game.state.add("step", step);
    game.state.start("step");
};

/**
 * Function to step forward (incrementally) through
 * animation. Updates the slider position accordingly.
 */
function forward() {
    if (selected_frame < max_frame && display_frame < max_frame) {
        selected_frame++;
        slider.value = display_frame + delta();
        update_frame();
    }
}

/**
 * Function to step backward (incrementally) through
 * animation. Updates the slider position accordingly.
 */
function backward() {
    if (selected_frame > 0) {
        selected_frame--;
        slider.value = display_frame + delta();
        update_frame();
    }
}

/**
 * Returns the delta between the display frame,
 * or current state of the board, and the selected,
 * or goal frame as defined by the user.
 */
function delta() {
    if (selected_frame < display_frame) {
        return -1;
    } else if (selected_frame > display_frame) {
        return 1;
    }
    return 0;
}

/**
 * Changes the state of the board by incrementing the
 * frame either forward or backward depending on the delta.
 */
function update_frame() {
    if (selected_frame >= 0 && selected_frame <= max_frame) {
        let d = delta();
        if (selected_frame == prev_json.length) {
            if (!server_closed) {
                fetch_data();
                slider.max = display_frame + d;
                slider.value = display_frame + d;
            }
        // data has already been fetched
        } else {
            if (!running) {
                running = true;
                animate(prev_json[display_frame + d]);
            }
        }
    }
}

/**
 * Takes information about a set of droplets and either
 * initializes the board or animates from the previous state.
 * @param {json} array of droplet json
 */
function parse_data(data, text_status) {
    var data = data.result;
    var jsons = [];
    for (let json of data) {
        // FIXME this will not work for multiple processes,
        // droplet id's will be the SAME!
        json.id = json.id.id;
        jsons.push(json);
    }

    prev_json.push(jsons);
    if (droplets.length == 0) {
        for (let json of jsons) {
            add_drop(json);
        }
    } else {
        if (!running) {
            animate(prev_json[display_frame]);
        }
    }
}

/**
 * Takes json for a single droplet and adds it to the board.
 * Initializes the associated tween and sprite and stores
 * each droplet by associated id in droplets.
 * @param {json} single droplet json
 */
function add_drop(json) {
    let s = game.add.sprite((json.location.x * CELL_SIZE) + CELL_SIZE,
        (json.location.y * CELL_SIZE)+ CELL_SIZE);
    let graphics = game.add.graphics(0, 0);
    let width = Math.sqrt(json.volume) * CELL_SIZE * json.dimensions.x;
    let height = Math.sqrt(json.volume) * CELL_SIZE * json.dimensions.y;
    graphics.beginFill(0x006699)
        .drawRoundedRect(0, 0, width, height, width)
        .endFill();
    s.addChild(graphics);
    let tween = game.add.tween(s);
    let drop = {
        sprite: s,
        deleted: false,
        id: json.id,
        volume: json.volume,
        info: json.info
    };
    droplets[json.id] = drop;
}

/**
 * Takes json for a set of droplets and creates a tween
 * from their previous state to the next one, removing
 * drops and creating combinations along the way.
 * Assigns the onComplete function to the first drop
 * in every set of droplets.
 * @param {data} array of droplet json
 */
function animate(data) {
    if (data != undefined) {
        remove_drops(data);
        let count = 0;
        for (let json of data) {
            let drop = droplets[json.id];

            if (drop == null) {
                add_drop(json);
            }

            if (drop != null) {
                if (drop.deleted) {
                    drop.sprite.revive();
                    drop.deleted = false;
                }
                let x = json.location.x * CELL_SIZE + CELL_SIZE;
                let y = json.location.y * CELL_SIZE + CELL_SIZE;
                let tween = game.add.tween(drop.sprite)
                    .to({ x: x, y: y },
                        TWEEN_TIME / (Math.abs(selected_frame - display_frame) + 1),
                        Phaser.Easing.Quadratic.InOut).start()
                if (count == 0) {
                    tween.onComplete.add(onComplete, this);
                }
                count++;
            }
        }
        display_frame += delta();
    }
}

/**
 * Removes drops that have been combined from one state
 * to the next.
 * @param {data} array of droplet json
 */
function remove_drops(data) {
    let new_ids = [];
    if (data != undefined && data.length > 0) {
        for (let id of data) {
            new_ids.push(id.id);
        }
        for (let droplet of droplets) {
            if (droplet != undefined) {
                if (new_ids.indexOf(droplet.id) == -1) {
                    droplets[droplet.id].sprite.kill();
                    droplets[droplet.id].deleted = true;
                }
            }
        }
    }
}

/**
 * Function that begins operations for the next set of
 * droplets once the previous set has completed.
 */
function onComplete() {
    if (selected_frame == display_frame) {
        running = false;
    } else {
        running = true;
        animate(prev_json[display_frame + delta()])
    }
}

/**
 * Begins execution whenever the 'ready' button is clicked
 * and runs continuously on a 500 ms interval until it's
 * clicked again.
 */
function run_animation() {
    let interval_id = setInterval(function() {
        if (ready && !server_closed) {
            forward();
        } else {
            clearInterval(interval_id);
        }
    }, 500);
}

$(function() {
    function jQuery_fetch(){
        var req_data = {
            jsonrpc: '2.0',
            id: 1000, // FIXME increment this
            method: 'visualizer_droplet_info',
            params: []
        };
        var fetch =
            $.ajax({
                url:'/rpc',
                type:"POST",
                data: JSON.stringify(req_data),
                contentType:"application/json; charset=utf-8",
                dataType:"json",
                success: parse_data
            })
            .fail(function() {
                server_closed = true;
                max_frame = display_frame;
                selected_frame = max_frame;
                display_frame = max_frame;
                running = false;
                slider.max = display_frame;
            });
    }
    fetch_data = jQuery_fetch;
});
