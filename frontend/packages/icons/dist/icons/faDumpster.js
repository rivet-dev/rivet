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

// src/node_modules/@fortawesome/free-solid-svg-icons/faDumpster.js
var require_faDumpster = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faDumpster.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dumpster";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f793";
    var svgPathData = "M132.3 64L106.7 192 24 192c-13.3 0-24-10.7-24-24l0-4.1c0-2.6 .4-5.1 1.2-7.6L26.5 80.4C29.8 70.6 39 64 49.3 64l83 0zm23.4 128l25.6-128 82.7 0 0 128-108.3 0zM312 64l82.7 0 25.6 128-108.3 0 0-128zm131.7 0l83 0c10.3 0 19.5 6.6 22.8 16.4l25.3 75.9c.8 2.4 1.2 5 1.2 7.6l0 4.1c0 13.3-10.7 24-24 24l-82.7 0-25.6-128zM25.7 240l524.7 0c-.4 2.1-13.3 73.4-38.9 213.7-3 16.3-17.9 27.6-34.4 26.1S448 464.6 448 448l0-16-320 0 0 16c0 16.6-12.6 30.4-29.1 31.9S67.5 470 64.5 453.7C39 313.3 26 242.1 25.7 240z";
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
    exports.faDumpster = exports.definition;
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

// src/_entries/faDumpster.js
var import_faDumpster = __toESM(require_faDumpster());
var export_faDumpster = import_faDumpster.definition;
export {
  export_faDumpster as faDumpster
};
