$fn=32;

// mount plate 
difference() {
    linear_extrude(3) {
        difference() {
            difference() {
                circle(52/2);
                circle(6/2);
            }

            union() {
                rotate([0,0,120]) {
                    translate([6,0,0]) {
                        circle(1.5);
                    }
                }
                rotate([0,0,240]) {
                    translate([6,0,0]) {
                        circle(1.5);
                    }
                }
                rotate([0,0,0]) {
                    translate([6,0,0]) {
                        circle(1.5);
                    }
                }
            }
        }
    }
    
    // "decorative" openings for material saving
    union() {
        for (j = [0:3]) {
            for (i = [60:1:60+45]) {
                rotate([0,0,i + (j*120)]) {
                    translate([16,0,0]) {
                        cylinder(10, 3, 3);
                    }
                }
            }
        }
    }
}

difference() {
    // outer walls
    translate([0,0,3]) {
        linear_extrude(7) {
            difference() {
                circle(52/2);
                circle(49/2);
            }
        }
    }

    // ribbon outlet
    translate([22,-3,5]) {
        cube([6,6,2]);
    }
}