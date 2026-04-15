var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/free-solid-svg-icons/faSeptagon.js
var require_faSeptagon = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSeptagon.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "septagon";
    var width = 576;
    var height = 512;
    var aliases = ["heptagon"];
    var unicode = "e820";
    var svgPathData = "M267.4-31.5c15.9-5.5 33.5-4.6 48.8 2.7l172.4 83 6.3 3.5c14.2 9 24.5 23.3 28.3 39.9l42.6 186.5 1.2 7.2c1.6 14.3-1.7 28.8-9.4 41.1l-4.2 5.9-119.3 149.6c-12.1 15.2-30.6 24.1-50 24.1l-191.3 0c-19.5 0-37.9-8.9-50-24.1L23.5 338.4c-12.1-15.2-16.7-35.2-12.3-54.2l42.6-186.5 2-7c5.6-15.9 17.2-29 32.6-36.4l172.4-83 6.7-2.7z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faSeptagon = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faHeptagon.js
var import_faSeptagon = __toESM(require_faSeptagon());
var export_faHeptagon = import_faSeptagon.definition;
export {
  export_faHeptagon as faHeptagon
};
