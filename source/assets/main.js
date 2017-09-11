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
                div.style.opacity = 0.05;
                main.style.backgroundColor = 'rgba(0, 0, 0, 0.8)';

            }, 1000);
        }

        setTimeout(function() {
            draw_strokes(svg, after_draw, true);
        }, 200);
    };

    xhr.send();
}).call(this);
