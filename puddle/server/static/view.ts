
const CELL_SIZE = 50;

interface DropletJSON {
    id: number;
    location: [number, number];
    volume: number;
    info: string;
    destination: [number, number];
}

// Have Droplet (the class) actually inherit the fields from DropletJSON
interface Droplet extends DropletJSON {}

class Droplet implements DropletJSON {

    constructor(json: DropletJSON) {
        console.log(`${frame} - creating`, json)
        this.id = json.id;
        this.location = json.location;
        this.volume = json.volume;
        this.info = json.info;
        if (json.destination != null) {
            this.destination = json.destination;
        }
    }

    render() {
        let r = Math.sqrt(this.volume) * CELL_SIZE / 2;
        let cr = CELL_SIZE / 2;
        let y = this.location[0] * CELL_SIZE + cr - r;
        let x = this.location[1] * CELL_SIZE + cr - r;

        context.beginPath();
        context.arc(x + (CELL_SIZE / 2), y + (CELL_SIZE / 2), r, 0, Math.PI * 2, false);
        context.closePath();
        context.fillStyle = "#006699";
        context.fill();
    }
}

let frame = 0;
let canvas: HTMLCanvasElement = <HTMLCanvasElement>document.getElementById('chip');
let context: CanvasRenderingContext2D = canvas.getContext("2d")

let frameArray = new Array<Array<Droplet>>(); // drops for each frame
let prevArray: Droplet[]; // drops from most recent frame

let duration = 1000; // animation length in ms
let startTime: number; // browser time at start of animation

let animate = true; // depends on checkbox

// TODO(@chrstn): put the init stuff in a separate place
function parse_data(data: DropletJSON[]) {
    frame += 1;

    context.clearRect(0, 0, canvas.width, canvas.height);
    frameArray[frame] = new Array<Droplet>();

    for (let json of data) {
        let droplet = new Droplet(json);
        frameArray[frame][droplet.id] = droplet;
    }

    if (frame != 1 && animate) {
        prevArray = frameArray[frame - 1].slice(0);
        requestAnimationFrame(anim);
    } else {
        prevArray = frameArray[frame].slice(0);
        draw();
    }
}

// Canvas animation using current browser time
function anim(time: number) {
    if (!startTime) {
        startTime = time || performance.now();
    }

    var delta = (time - startTime) / duration;

    for (let i = 0; i < prevArray.length; i++) {
        if (prevArray[i] != null && frameArray[frame][i] != null) {
            let deltaX = (frameArray[frame][i].location[0] -
                frameArray[frame - 1][i].location[0]) * delta;
            let deltaY = (frameArray[frame][i].location[1] -
                frameArray[frame - 1][i].location[1]) * delta;
            prevArray[i].location[0] += deltaX;
            prevArray[i].location[1] += deltaY;
        }
    }

    if (delta >= 1) { // animation complete
        startTime = null;
        draw();
    } else {
        draw();
        requestAnimationFrame(anim);
    }
}

// Draws all elements of prevArray
function draw(){
    context.clearRect(0, 0, canvas.width, canvas.height);
    for (let i = 0; i < prevArray.length; i++) {
        if (prevArray[i] != null) {
           var drop = prevArray[i];
           drop.render();
        }
    }
}

// button interactions
// TODO(@chrstn): work out behavior for rapid progression/animations
function get_data() {
    if ($("#ready").is(':checked')) {
        setTimeout(() => {
            $.getJSON('/state', parse_data).done(get_data)
        }, 200)
    }
}

$("#step").click(() => {
    $.getJSON('/state', parse_data)
});

$("#ready").change(
    function() {
        if ($(this).is(':checked')) {
            get_data()
        }
    });

if ($("#ready").is(':checked')) {
    get_data()
}

$('#animate').change(function(){
    animate = $(this).is(':checked');
});
