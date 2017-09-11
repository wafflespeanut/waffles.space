const TYPE_INTERVAL = 25;

(function() {
    function setup_strokes(node, path_delay) {
        var delay = 0;
        var trans_timeout = 0;
        var paths = node.querySelectorAll('path');

        for (i = paths.length - 1; i >= 0; i--) {
            delay += path_delay;
            paths[i].style.transition = 'opacity ' + path_delay + 'ms ' + delay + 'ms linear';
        }

        node.draw_time = delay;
    }

    function draw_strokes(node, callback, call_on_end) {
        var paths = node.querySelectorAll('path');
        for (i = 0; i < paths.length; i++) {
            paths[i].style.opacity = 1;
        }

        if (callback) {
            if (call_on_end) {
                setTimeout(callback, node.draw_time);
            } else {
                callback();
            }
        }
    }

    var div = document.getElementById('smiley');
    var main = document.getElementsByTagName('main')[0];
    var img = document.createElement('img');
    img.src = 'assets/smiley.png';

    let lines = [
        "You've been thrown into an abandoned room...",
        "A guardian's next to the door. He's been looking after this room for ages.",
        "... and he's woken up by your arrival.",
        "",
        "Here's what you can do..."
    ];

    var choices = [
        {
            "m": "Hear the guardian's stories.",
            "l": "https://blog.waffles.space/"
        },
        {
            "m": "See what the guardian can do.",
            "l": "https://waffles.space/resume/"
        }
    ];

    var xhr = new XMLHttpRequest();
    xhr.open('get', 'assets/smiley.svg', true);
    xhr.onreadystatechange = function() {
        if (xhr.readyState != 4) {
            return
        }

        var svg = xhr.responseXML.documentElement;
        svg = document.importNode(svg, true);
        div.appendChild(svg);
        div.appendChild(img);
        svg.setAttribute('viewBox', '0 0 400 400');
        setup_strokes(svg, 20);

        function after_draw() {
            img.style.opacity = 1;
            svg.style.opacity = 0;

            setTimeout(function() {
                div.removeChild(svg);
                div.style.opacity = 0;
                main.style.backgroundColor = 'rgba(0, 0, 0, 0.9)';

                var writer = document.getElementById('writer');
                var p = document.createElement('p');
                writer.appendChild(p);
                var i = 0, j = 0;
                var interval_id;

                function print_lines(finish_callback) {
                    console.log(i);
                    if (i < lines.length) {
                        if (j < lines[i].length) {
                            p.innerHTML += lines[i][j];
                            j += 1;
                        } else {
                            j = 0;
                            clearInterval(interval_id);

                            setTimeout(function() {
                                i += 1;
                                p.className = ' stop-blink';
                                p = document.createElement('p');
                                writer.appendChild(p);
                                interval_id = setInterval(function() {
                                    print_lines(finish_callback);
                                }, TYPE_INTERVAL);
                            }, 500);
                        }
                    } else {
                        clearInterval(interval_id);
                        if (finish_callback) {
                            finish_callback();
                        }
                    }
                }

                var choices_div = document.createElement('div');
                choices_div.id = 'choices';

                for (k = 0; k < choices.length; k++) {
                    var choice = document.createElement('p');
                    choice.innerHTML = choices[k].m;
                    choice.className = 'link';
                    choice.link = choices[k].l;
                    choice.addEventListener('click', function() {
                        window.location = this.link;
                    }, false);

                    choices_div.appendChild(choice);
                }

                interval_id = setInterval(function() {
                    print_lines(function() {
                        writer.appendChild(choices_div);
                    });
                }, TYPE_INTERVAL);
            }, 1000);
        }

        setTimeout(function() {
            draw_strokes(svg, after_draw, true);
        }, 200);
    };

    xhr.send();
}).call(this);
