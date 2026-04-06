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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCircleArrowDown.js
var require_faCircleArrowDown = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCircleArrowDown.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-arrow-down";
    var width = 512;
    var height = 512;
    var aliases = ["arrow-circle-down"];
    var unicode = "f0ab";
    var svgPathData = "M256 512a256 256 0 1 0 0-512 256 256 0 1 0 0 512zM239 377l-80-80c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l39 39 0-150.1c0-13.3 10.7-24 24-24s24 10.7 24 24l0 150.1 39-39c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9l-80 80c-9.4 9.4-24.6 9.4-33.9 0z";
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
    exports.faCircleArrowDown = exports.definition;
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

// src/_entries/faArrowCircleDown.js
var import_faCircleArrowDown = __toESM(require_faCircleArrowDown());
var export_faArrowCircleDown = import_faCircleArrowDown.definition;
export {
  export_faArrowCircleDown as faArrowCircleDown
};
