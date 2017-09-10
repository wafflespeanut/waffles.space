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
    var img = document.createElement('img');
    img.src = 'assets/smiley.png';

    var xhr = new XMLHttpRequest();
    xhr.open('get', 'assets/smiley.svg', true);
    xhr.onreadystatechange = function() {
        if (xhr.readyState != 4) {
            return
        }

        var svg = xhr.responseXML.documentElement;
        svg = document.importNode(svg, true);
        setup_strokes(svg, 25);
        div.appendChild(svg);
        div.appendChild(img);

        setTimeout(function() {
            draw_strokes(svg, function() {
                img.style.opacity = 1;
                svg.style.opacity = 0;

                setTimeout(function() {
                    div.removeChild(svg);
                }, 1000);
            }, true);
        }, 200);
    };

    xhr.send();
}).call(this);
