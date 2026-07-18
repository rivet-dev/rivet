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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMasksTheater.js
var require_faMasksTheater = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMasksTheater.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "masks-theater";
    var width = 576;
    var height = 512;
    var aliases = [127917, "theater-masks"];
    var unicode = "f630";
    var svgPathData = "M-5 118L23.5 279.7c14 79.5 76.3 141.8 155.8 155.8l12.7 2.2c-16.5-28.6-27.1-60.7-30.6-94.5l-24.1 4.3c-9.7 1.7-18.8-5.8-16.9-15.5 4.8-24.7 19.1-46.6 39.7-60.9l0-74.6c-1.4 .8-3 1.3-4.7 1.6l-63 11.1c-8.7 1.5-17.3-4.4-15.9-13.1 3.1-19.6 18.4-36 39.1-39.7 17.2-3 33.9 3.5 44.6 15.8l0-22.7c0-22.5 6.9-52.4 32.3-73.4 26-21.5 67.7-43.9 124.9-54.2-30.5-16.3-86.3-32-163.8-18.4-80.3 14.2-128 50.1-150.1 76.1-9 10.5-10.8 24.9-8.4 38.5zM208 138.7l0 174.8c0 80.7 50.5 152.9 126.4 180.4L362.1 504c14.1 5.1 29.6 5.1 43.7 0L433.6 494C509.5 466.4 560 394.3 560 313.5l0-174.8c0-6.9-2.1-13.8-7-18.6-22.6-22.5-78.2-56-169-56s-146.4 33.6-169 56c-4.9 4.9-7 11.7-7 18.6zm66.1 187.1c-1.4-7 7-11 12.7-6.6 26.9 20.6 60.6 32.9 97.2 32.9s70.2-12.3 97.2-32.9c5.7-4.4 14.1-.4 12.7 6.6-10.1 51.4-55.5 90.3-109.9 90.3s-99.8-38.8-109.9-90.3zm.5-101.5C281.2 205.5 299 192 320 192s38.9 13.5 45.4 32.3c2.9 8.4-4.5 15.7-13.4 15.7l-64 0c-8.8 0-16.3-7.4-13.4-15.7zM480 240l-64 0c-8.8 0-16.3-7.4-13.4-15.7 6.5-18.8 24.4-32.3 45.4-32.3s38.9 13.5 45.4 32.3c2.9 8.4-4.5 15.7-13.4 15.7z";
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
    exports.faMasksTheater = exports.definition;
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

// src/_entries/faMasksTheater.js
var import_faMasksTheater = __toESM(require_faMasksTheater());
var export_faMasksTheater = import_faMasksTheater.definition;
export {
  export_faMasksTheater as faMasksTheater
};
