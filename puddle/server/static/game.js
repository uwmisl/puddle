let fetch_data = null;
let game;
let droplets = [];

const CELL_SIZE = 50;
const Y_OFFSET = 100; // downward offset to leave room for 'step' btn

/**
 * Loads all of the necessary Phaser stuff and initializes
 * the step function.
 * TODO: ensure devicePixelRatio takes care of high-res screens
 * TODO: add automatic step-through checkbox/function
 * TODO: add tween when volume increases
 */
window.onload = function() {
    game = new Phaser.Game(
        window.innerWidth * window.devicePixelRatio, 
        window.innerHeight * window.devicePixelRatio, 
        Phaser.CANVAS, 
        'gameArea');
    var step = function(game) {}

    step.prototype = {
        // load assets here if we wind up with any
        preload: function() {},

        create: function() {
            game.stage.backgroundColor = "#ffffff";
            game.physics.startSystem(Phaser.Physics.ARCADE);

            fetch_data();

            var button = new Phaser.Graphics(game)
                .beginFill(0x00, .5)
                .drawRoundedRect(10, 50, 100, 40, 10)
                .endFill()
                .generateTexture();

            var buttonHover = new Phaser.Graphics(game)
                .beginFill(0x00, .8)
                .drawRoundedRect(10, 50, 100, 40, 10)
                .endFill()
                .generateTexture();

            var text = this.game.add.text(25, 8, "STEP",
                {font: "20px Arial", fill: "#ffffff"});

            this.step_button = this.add.image(0, 0, button);
            this.step_button.addChild(text);

            this.step_button.inputEnabled = true;
            this.step_button.input.useHandCursor = true;

            this.step_button.events.onInputDown.add(function() {
                fetch_data();
            }, this);

            this.step_button.events.onInputOver.add(function() {
                this.step_button.setTexture(buttonHover);
            } , this);

            this.step_button.events.onInputOut.add(function() {
                this.step_button.setTexture(button);
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
    var s = game.add.sprite((json.location[1] * CELL_SIZE), (json.location[0] * CELL_SIZE) + Y_OFFSET);
    var graphics = game.add.graphics(0, 0);
    graphics.beginFill(0x006699)
        .drawCircle(CELL_SIZE / 2, CELL_SIZE / 2, (Math.sqrt(json.volume) * CELL_SIZE))
        .endFill();
    s.addChild(graphics);
    let tween = game.add.tween(s);
    var drop = {
        sprite: s,
        last_added_tween: tween.start(),
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
    removeDrops(data);
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
function removeDrops(data) {
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

$(function() {
    function jQuery_fetch(){
        $.getJSON('/state', parse_data);
    }
    fetch_data = jQuery_fetch;
});
