let fetch_data = null;
let sprites = [];
let game;
const CELL_SIZE = 50;
let frame = 0;
let frames = [[[]]];

window.onload = function() {
    game = new Phaser.Game(600, 600, Phaser.CANVAS);
    var step = function(game) {}

    step.prototype = {
        // load assets here if we wind up with any
        preload: function() {},

        create: function() {
            game.stage.backgroundColor = "#ffffff";
            game.physics.startSystem(Phaser.Physics.ARCADE);

            fetch_data();

            var button = new Phaser.Graphics()
                .beginFill(0x00, .3)
                .drawRoundedRect(10, 50, 100, 40, 10)
                .endFill()
                .generateTexture();

            var buttonHover = new Phaser.Graphics()
                .beginFill(0x00, .4)
                .drawRoundedRect(10, 50, 100, 40, 10)
                .endFill()
                .generateTexture();

            var text = this.game.add.text(25, 8, "step",
                {font: "bold 20px Courier", fill: "#000000"});

            this.stepButton = this.add.image(0, 0, button);
            this.stepButton.addChild(text);

            this.stepButton.inputEnabled = true;
            this.stepButton.input.useHandCursor = true;

            this.stepButton.events.onInputDown.add(function() {
                fetch_data();
            }, this);

            this.stepButton.events.onInputOver.add(function() {
                this.stepButton.setTexture(buttonHover);
            } , this);

            this.stepButton.events.onInputOut.add(function() {
                this.stepButton.setTexture(button);
            } , this);

            game.stage.disableVisibilityChange = true;
        },

        update:function() {},

    }

    game.state.add("step", step);
    game.state.start("step");
}

function parse_data(data) {
    console.log(data);
    if (sprites.length == 0) {
        init_board(data);
    } else {
        animate(data);
    }
    frame += 1;
    console.log(sprites);
}

function init_board(data) {
    let snapshot = [];
    for (let json of data) {
        snapshot[json.id] = json.location;
        sprites[json.id] = game.add.sprite(json.location[0]*CELL_SIZE, json.location[1]*CELL_SIZE);
        var graphics = game.add.graphics(0, 0);
        graphics.beginFill(0xFF3300)
            .drawCircle(20, 20, 50)
            .endFill();
        sprites[json.id].addChild(graphics);
    }
    frames[frame] = snapshot;
}

function animate(data) {
    let snapshot = [];
    for (let json of data) {
        console.log(snapshot);
        snapshot[json.id] = json.location;
        console.log(frames[frame - 1][json.id][0] * CELL_SIZE);
        // .from({x: (frames[frame - 1][json.id][0] * CELL_SIZE), y: (frames[frame - 1][json.id][1]* CELL_SIZE)}).
        game.add.tween(sprites[json.id]).to({ x: json.location[0]*CELL_SIZE, y: json.location[1]*CELL_SIZE }, 500, Phaser.Easing.Linear.None, true);
    }
    frames[frame] = snapshot;

}

$(function() {
    function jQuery_fetch(){
        $.getJSON('/state', parse_data)
    }
    fetch_data = jQuery_fetch;
});
