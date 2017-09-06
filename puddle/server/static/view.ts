
const CELL_SIZE = 50;

interface DropletJSON {
    id: number;
    x: number;
    y: number;
    info: string;
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
        this.x = json.x;
        this.y = json.y;
        this.info = json.info;
    }

    // gets the HTML node for this droplet, creating it if necessary
    get node(): JQuery<HTMLElement> {
        let node = $('#' + this.id);
        if (node.length > 0)
            return node

        node = $(`<div id="${this.id}" class="ball"></div>`);
        node.appendTo($('#container'))
        node.css('border-radius', CELL_SIZE)
        node.css('height', CELL_SIZE)
        node.css('width', CELL_SIZE)

        return node
    }

    render() {
        this.node.text(this.info)
        let x = this.x * CELL_SIZE;
        let y = this.y * CELL_SIZE;
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
