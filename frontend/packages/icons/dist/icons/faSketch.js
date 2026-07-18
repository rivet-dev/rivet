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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSketch.js
var require_faSketch = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSketch.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "sketch";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f7c6";
    var svgPathData = "M27.8 162.2l-18.5 24.9 90.5 0 6.9-130.7-78.9 105.8zM396.6 45.7L268 32 403.7 179.2 396.6 45.7zM112.5 218.3l-11.2-22-91.1 0 224.9 261.7-122.6-239.7zm2-31.2l284 0C340.6 124.3 293.3 72.9 256.6 33L114.5 187.1zm297.3 9.1l-133.9 261.8 224.8-261.7-90.9 0 0-.1zM415.7 69l-9.4-12.6 .9 17.3 6.1 113.4 90.3 0-87.9-118.1zM113.8 93.5L109.2 179.1 245 32 116.4 45.7 113.8 93.5zM401.5 196.2l-290 0c18.2 35.6 66.6 130.2 145.1 283.8L401.5 196.2z";
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
    exports.faSketch = exports.definition;
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

// src/_entries/faSketch.js
var import_faSketch = __toESM(require_faSketch());
var export_faSketch = import_faSketch.definition;
export {
  export_faSketch as faSketch
};
