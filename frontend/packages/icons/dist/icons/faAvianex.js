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

// src/node_modules/@fortawesome/free-brands-svg-icons/faAvianex.js
var require_faAvianex = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faAvianex.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "avianex";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f374";
    var svgPathData = "M453.2 32l-312 0C102.3 32 65 63.2 57.9 101.7L1.3 410.3C-5.8 448.8 20 480 59 480l312 0c38.9 0 76.2-31.2 83.3-69.7L511 101.8C518 63.2 492.2 32 453.2 32zM395 379.3l-32 13.5-115.4-110c-14.7 10-29.2 19.5-41.7 27.1l22.1 64.2-17.9 12.7-40.6-61-52.4-48.1 15.7-15.4 58 31.1c9.3-10.5 20.8-22.6 32.8-34.9l-20.5-29.6-68.8-99.8 18.8-28.9 8.9-4.8 103.1 112.4 4.9 4.5c19.4-18.8 33.8-32.4 33.8-32.4 7.7-6.5 21.5-2.9 30.7 7.9 9 10.5 10.6 24.7 2.7 31.3-1.8 1.3-15.5 11.4-35.3 25.6l4.5 7.3 94.9 119.4-6.3 7.9z";
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
    exports.faAvianex = exports.definition;
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

// src/_entries/faAvianex.js
var import_faAvianex = __toESM(require_faAvianex());
var export_faAvianex = import_faAvianex.definition;
export {
  export_faAvianex as faAvianex
};
