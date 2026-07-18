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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBoxTissue.js
var require_faBoxTissue = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBoxTissue.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "box-tissue";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e05b";
    var svgPathData = "M103.9 32l161 0c13.8 0 26 8.8 30.4 21.9l17.4 52.2c4.4 13.1 16.6 21.9 30.4 21.9l60.5 0c21.8 0 37.3 21.4 30.4 42.1L384 320 128 320 72.7 70.9C68.2 51 83.4 32 103.9 32zM48 256l16.6 0 16.5 74.4C86 352.4 105.5 368 128 368l256 0c20.7 0 39-13.2 45.5-32.8l26.4-79.2 8.1 0c26.5 0 48 21.5 48 48l0 128c0 26.5-21.5 48-48 48L48 480c-26.5 0-48-21.5-48-48L0 304c0-26.5 21.5-48 48-48z";
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
    exports.faBoxTissue = exports.definition;
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

// src/_entries/faBoxTissue.js
var import_faBoxTissue = __toESM(require_faBoxTissue());
var export_faBoxTissue = import_faBoxTissue.definition;
export {
  export_faBoxTissue as faBoxTissue
};
