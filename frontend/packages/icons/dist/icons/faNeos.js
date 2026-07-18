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

// src/node_modules/@fortawesome/free-brands-svg-icons/faNeos.js
var require_faNeos = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faNeos.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "neos";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f612";
    var svgPathData = "M383.8 512l-95.1 0-108.2-154.5 0 91.1-86.4 63.4-97.7 0 0-482.2 40.5-29.8 108 0 123.7 176.1 0-112.7 86.4-63.4 97.7 0 0 461.5-68.9 50.5zM7.2 35.3l0 460.7 72-52.9 0-249.1 215.5 307.6 84.8 0 52.4-38.2-78.3 0-316.1-450.5-30.2 22.3zM89.7 501.9l80-58.8 0-101-79.8-114.4 0 220.9-72.6 53.3 72.3 0 0 0zM49 10.8l310.6 442.6 82.4 0 0-442.6-79.8 0 0 317.6-222.9-317.6-90.3 0zM279.4 191.6l72 102.8 0-278.5-72 53 0 122.7z";
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
    exports.faNeos = exports.definition;
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

// src/_entries/faNeos.js
var import_faNeos = __toESM(require_faNeos());
var export_faNeos = import_faNeos.definition;
export {
  export_faNeos as faNeos
};
