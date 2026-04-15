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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSith.js
var require_faSith = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSith.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "sith";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f512";
    var svgPathData = "M0 32l69.7 118.8-58.9-11.5 69.8 91c-3 17-3 34.4 0 51.4l-69.8 91 58.9-11.5-69.7 118.8 118.8-69.7-11.5 58.9 91-69.8c17 3 34.5 3 51.5 0l91 69.8-11.5-58.9 118.7 69.7-69.7-118.8 58.9 11.5-69.8-91c3-17 3-34.4 0-51.4l69.8-91-58.9 11.5 69.7-118.8-118.7 69.7 11.5-58.9-91.1 69.9c-8.5-1.5-17.1-2.3-25.7-2.3s-17.2 .8-25.7 2.3L107.2 42.8 118.8 101.7 0 32zM224 380.2a124.2 124.2 0 1 1 0-248.4 124.2 124.2 0 1 1 0 248.4zm0-211.8a87.6 87.6 0 1 0 0 175.1 87.6 87.6 0 1 0 0-175.1z";
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
    exports.faSith = exports.definition;
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

// src/_entries/faSith.js
var import_faSith = __toESM(require_faSith());
var export_faSith = import_faSith.definition;
export {
  export_faSith as faSith
};
