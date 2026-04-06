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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHelicopter.js
var require_faHelicopter = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHelicopter.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "helicopter";
    var width = 576;
    var height = 512;
    var aliases = [128641];
    var unicode = "f533";
    var svgPathData = "M176 32c-13.3 0-24 10.7-24 24s10.7 24 24 24l152 0 0 48-220.8 0-32.8-39.4C69.9 83.2 63.1 80 56 80L24 80C15.7 80 8 84.3 3.6 91.4s-4.8 15.9-1.1 23.4l48 96C54.6 218.9 62.9 224 72 224l107.8 0 104 143.1c15.1 20.7 39.1 32.9 64.7 32.9l75.5 0c75.1 0 136-60.9 136-136S499.1 128 424 128l-48 0 0-48 152 0c13.3 0 24-10.7 24-24s-10.7-24-24-24L176 32zM376 192l48 0c39.8 0 72 32.2 72 72s-32.2 72-72 72l-48 0 0-144zM552 416c-13.3 0-24 10.7-24 24 0 4.4-3.6 8-8 8l-272 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l272 0c30.9 0 56-25.1 56-56 0-13.3-10.7-24-24-24z";
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
    exports.faHelicopter = exports.definition;
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

// src/_entries/faHelicopter.js
var import_faHelicopter = __toESM(require_faHelicopter());
var export_faHelicopter = import_faHelicopter.definition;
export {
  export_faHelicopter as faHelicopter
};
