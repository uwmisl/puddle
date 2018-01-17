
const CELL_SIZE = 50;

interface DropletJSON {
    id: number;
    location: [number, number];
    volume: number;
    info: string;
    shape: [number, number][];
}

let droplets = new Map<number, Droplet>();

// Have Droplet (the class) actually inherit the fields from DropletJSON
interface Droplet extends DropletJSON {}

class Droplet implements DropletJSON {

    constructor(json: DropletJSON) {
        console.log(`${counter} - creating`, json)
        this.id = json.id;
        this.update(json)
        droplets.set(this.id, this)
    }

    update(json: DropletJSON) {
        if (this.id != json.id) {
            console.error('updating with droplet that has the wrong id', this, json)
        }
        this.location = json.location;
        this.volume = json.volume;
        this.info = json.info;
        this.shape = json.shape;
    }

    // gets the HTML node for this droplet, creating it if necessary
    get node(): JQuery<HTMLElement> {
        let node = $('#' + this.id);
        if (node.length > 0)
            return node

        console.log("Creating node for ", this.id)

        node = $(`<div id="${this.id}" class="ball-container"></div>`);
        node.appendTo($('#container'))

        for (let cell of this.shape) {
            console.log("cell: ", cell)
            let y = cell[0] * CELL_SIZE;
            let x = cell[1] * CELL_SIZE;

            let cell_node = $(`<div class="ball"></div>`);
            cell_node.css('top', y);
            cell_node.css('left', x);

            let r = CELL_SIZE / 2;
            cell_node.css('border-radius', r)
            cell_node.css('height', r * 2)
            cell_node.css('width', r * 2)

            cell_node.appendTo(node);
            console.log("cell node ", cell_node)
            console.log("node ", node)
        }

        let r = Math.sqrt(this.volume) * CELL_SIZE / 2;
        node.css('border-radius', r)
        node.css('height', r * 2)
        node.css('width', r * 2)

        return node
    }

    get text_node() {
        let id = this.id + '-text'
        let text_node = $("#" + id);
        if (text_node.length > 0)
            return text_node

        text_node = $(`<div id="${id}"></div>`)
        text_node.appendTo(this.node)

        return text_node;
    }

    render() {
        this.text_node.text(this.info)
        let r = Math.sqrt(this.volume) * CELL_SIZE / 2;
        let cr = CELL_SIZE / 2;
        let y = this.location[0] * CELL_SIZE + cr - r;
        let x = this.location[1] * CELL_SIZE + cr - r;
        this.node.css('transform', `translate(${x}px, ${y}px)`);
    }

    destroy() {
        console.log(`${counter} - deleting`, this)
        this.node.remove()
        droplets.delete(this.id)
    }
}

let counter = 0

function parse_data(data: DropletJSON[]) {

    counter += 1;

    // remove old droplets
    for (let [id, droplet] of droplets) {
        let is_present = data.find((elem) => elem.id == id)
        if (!is_present) {
            droplet.destroy()
        }
    }

    // add or update droplets
    for (let json of data) {
        let droplet = droplets.get(json.id)
        if (!droplet) {
            droplet = new Droplet(json)
        } else {
            droplet.update(json)
        }
        droplet.render()
    }
}

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
