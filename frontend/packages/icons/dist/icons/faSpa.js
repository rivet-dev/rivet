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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSpa.js
var require_faSpa = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSpa.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "spa";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f5bb";
    var svgPathData = "M169.7 227.5c34.3 18.7 64.5 44.1 88.8 74.5 11.1 13.9 21 28.9 29.4 44.8 8.5-15.9 18.3-30.8 29.4-44.8 24.3-30.5 54.5-55.9 88.8-74.5 41.7-22.7 89.4-35.5 139.7-35.5l9.9 0c11.1 0 20.1 9 20.1 20.1 0 148-119.9 267.9-267.9 267.9l-40.2 0C119.9 480 0 360.1 0 212.1 0 201 9 192 20.1 192l9.9 0c50.4 0 98.1 12.8 139.7 35.5zM303.9 38c16.9 15.9 61.4 63.4 89 142.3-40.1 20.2-75.8 48.2-104.9 82.1-29.2-33.9-64.8-61.8-104.9-82.1 27.6-78.9 72.2-126.4 89-142.3 4.3-4.1 10-6 15.9-6s11.6 2 15.9 6z";
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
    exports.faSpa = exports.definition;
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

// src/_entries/faSpa.js
var import_faSpa = __toESM(require_faSpa());
var export_faSpa = import_faSpa.definition;
export {
  export_faSpa as faSpa
};
