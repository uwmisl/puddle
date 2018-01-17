let fetch_data = null;
let game;
let droplets = [];

let ready = false;

const CELL_SIZE = 50;
const Y_OFFSET = 100; // downward offset to leave room for 'step' btn

/**
 * Loads all of the necessary Phaser stuff and initializes
 * the step function.
 * TODO: ensure devicePixelRatio takes care of high-res screens
 * TODO: add tween when volume increases
 */
window.onload = function() {
    game = new Phaser.Game(
        window.innerWidth * window.devicePixelRatio, 
        window.innerHeight * window.devicePixelRatio, 
        Phaser.CANVAS, 
        'gameArea');
    let step = function(game) {}

    step.prototype = {
        // load assets here if we wind up with any
        preload: function() {},

        create: function() {
            game.stage.backgroundColor = "#ffffff";
            game.physics.startSystem(Phaser.Physics.ARCADE);

            fetch_data();

            let button = new Phaser.Graphics(game)
                .beginFill(0x00, .5)
                .drawRoundedRect(10, 50, 100, 40, 10)
                .endFill()
                .generateTexture();

            let button_hover = new Phaser.Graphics(game)
                .beginFill(0x00, .8)
                .drawRoundedRect(10, 50, 100, 40, 10)
                .endFill()
                .generateTexture();

            let text = this.game.add.text(25, 8, "STEP",
                {font: "20px Arial", fill: "#ffffff"});

            this.step_button = this.add.image(0, 0, button);
            this.step_button.addChild(text);

            this.step_button.inputEnabled = true;
            this.step_button.input.useHandCursor = true;

            this.step_button.events.onInputDown.add(function() {
                fetch_data();
            }, this);

            this.step_button.events.onInputOver.add(function() {
                this.step_button.setTexture(button_hover);
            } , this);

            this.step_button.events.onInputOut.add(function() {
                this.step_button.setTexture(button);
            } , this);

            let box = new Phaser.Graphics(game)
                .drawRect(10, 50, 100, 40)
                .generateTexture();

            // TODO: make these buttons a little nicer looking
            let check_text = this.game.add.text(10, 8, "READY: [  ]",
                {font: "20px Arial", fill: "#000000"});

            let checked_text = this.game.add.text(93, 10, "",
                            {font: "20px Arial", fill: "#000000"});

            this.rapid_box = this.add.image(110, 0, box);
            this.rapid_box.addChild(check_text);

            this.rapid_box.inputEnabled = true;
            this.rapid_box.input.useHandCursor = true;
            this.rapid_box.addChild(checked_text);


            this.rapid_box.events.onInputDown.add(function() {
                if (!ready) {
                    checked_text.text = "X";
                    ready = true;
                    run_animation();
                } else {
                    checked_text.text = "";
                    ready = false;
                }
            }, this);

            this.rapid_box.events.onInputOver.add(function() {
            } , this);

            this.rapid_box.events.onInputOut.add(function() {
            } , this);

            game.stage.disableVisibilityChange = true;
        },

        update:function() {},
    }

    game.state.add("step", step);
    game.state.start("step");
}

/**
 * Takes information about a set of droplets and either
 * initializes the board or animates from the previous state.
 * @param {json} array of droplet json
 */
function parse_data(data) {
    if (droplets.length == 0) {
        for (let json of data) {
            add_drop(json);
        }
    } else {
        animate(data);
    }
}

/**
 * Takes json for a single droplet and adds it to the board.
 * Initializes the associated tween and sprite and stores
 * each droplet by associated id in droplets.
 * @param {json} single droplet json
 */
function add_drop(json) {
    console.log(`creating`, json);
    let s = game.add.sprite((json.location[1] * CELL_SIZE), (json.location[0] * CELL_SIZE) + Y_OFFSET);
    let graphics = game.add.graphics(0, 0);
    graphics.beginFill(0x006699)
        .drawCircle(CELL_SIZE / 2, CELL_SIZE / 2, Math.sqrt(json.volume) * CELL_SIZE)
        .endFill();
    s.addChild(graphics);
    let tween = game.add.tween(s).to({radius: (Math.sqrt(json.volume) * CELL_SIZE)}, 500).start();
    let drop = {
        sprite: s,
        last_added_tween: tween,
        last_run_tween: tween,
        to_delete: false,
        deleted: false,
        diff: 1,
        id: json.id,
        volume: json.volume,
        info: json.info,
        destination: json.destination
    }
    droplets[json.id] = drop;
}

/**
 * Takes json for a set of droplets and creates a tween
 * from their previous state to the next one, removing
 * drops and creating combinations along the way.
 * @param {data} array of droplet json
 */
function animate(data) {
    remove_drops(data);
    for (let json of data) {
        if (droplets[json.id] == null) {
            add_drop(json);
        }
        let tween = game.add.tween(droplets[json.id].sprite).to({ x: (json.location[1] * CELL_SIZE), 
                                    y: (json.location[0] * CELL_SIZE)  + 100}, 500 / droplets[json.id].diff);
        droplets[json.id].last_added_tween.chain(tween);
        if (droplets[json.id].last_run_tween == null) {
            droplets[json.id].last_run_tween = tween;
        }
        if (droplets[json.id].last_added_tween == droplets[json.id].last_run_tween) {
            tween.start().onComplete.add(onComplete, {
                'drop': droplets[json.id], 
                'tween': tween});
        } else {
            tween.onComplete.add(onComplete, {
                'drop': droplets[json.id], 
                'tween': tween});
            droplets[json.id].diff += 1;
        }
        droplets[json.id].last_added_tween = tween;
    }
}

/**
 * Removes drops that have been combined from one state
 * to the next.
 * @param {data} array of droplet json
 */
function remove_drops(data) {
    let new_ids = [];
    for (let id of data) {
        new_ids.push(id.id);
    }
    for (droplet of droplets) {
        if (droplet != undefined) {
            if (new_ids.indexOf(droplet.id) == -1) {
                if (droplets[droplet.id].diff == 1) {
                    droplets[droplet.id].sprite.kill();
                    droplets[droplet.id].deleted = true;
                } else {
                    droplets[droplet.id].to_delete = true;
                }
            }
        }
    }    
}

/**
 * Function used when 'step' is called even though
 * a tween is currenty running. Updates last_run_tween
 * and kills drops that have been combined.
 */
function onComplete() {
    this.drop.last_run_tween = this.tween;
    if (this.drop.diff > 1) {
        this.drop.diff--;
    } else if (this.drop.to_delete) {
        this.drop.sprite.kill();
        this.drop.to_delete = false;
        this.drop.deleted = true;
    }
}

/**
 * Begins execution whenever the 'ready' button is clicked
 * and runs continuously on a 500 ms interval until it's
 * clicked again. 
 * TODO: Figure out how to prevent interval from continuing 
 * once GET runs out of data to fetch.
 */
function run_animation() {
    let interval_id = setInterval(function() {
        if (ready) {
            fetch_data();
        } else {
            clearInterval(interval_id);
        }
    }, 500);
}

$(function() {
    function jQuery_fetch(){
        $.getJSON('/state', parse_data);
    }
    fetch_data = jQuery_fetch;
});
