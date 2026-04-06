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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTrowelBricks.js
var require_faTrowelBricks = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTrowelBricks.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "trowel-bricks";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e58a";
    var svgPathData = "M224 32c0-11.5-6.2-22.2-16.2-27.8s-22.3-5.5-32.2 .4l-160 96C5.9 106.3 0 116.8 0 128s5.9 21.7 15.5 27.4l160 96c9.9 5.9 22.2 6.1 32.2 .4S224 235.5 224 224l0-64 256 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-256 0 0-64zm96 192c-17.7 0-32 14.3-32 32l0 64c0 17.7 14.3 32 32 32l160 0c17.7 0 32-14.3 32-32l0-64c0-17.7-14.3-32-32-32l-160 0zM0 416l0 64c0 17.7 14.3 32 32 32l96 0c17.7 0 32-14.3 32-32l0-64c0-17.7-14.3-32-32-32l-96 0c-17.7 0-32 14.3-32 32zm224-32c-17.7 0-32 14.3-32 32l0 64c0 17.7 14.3 32 32 32l256 0c17.7 0 32-14.3 32-32l0-64c0-17.7-14.3-32-32-32l-256 0z";
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
    exports.faTrowelBricks = exports.definition;
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

// src/_entries/faTrowelBricks.js
var import_faTrowelBricks = __toESM(require_faTrowelBricks());
var export_faTrowelBricks = import_faTrowelBricks.definition;
export {
  export_faTrowelBricks as faTrowelBricks
};
