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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBaguette.js
var require_faBaguette = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBaguette.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "baguette";
    var width = 640;
    var height = 512;
    var aliases = [129366];
    var unicode = "e3d8";
    var svgPathData = "M583 234.5c58.8-39.2 74.7-118.7 35.5-177.5S499.8-17.7 441 21.5L356.5 77.8l70.8 70.8c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0L329.3 95.9l-88 58.7 74 74c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0l-78.6-78.6-88 58.7 77.2 77.2c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0L98.9 249.5 57 277.5C-1.8 316.7-17.7 396.2 21.5 455s118.7 74.7 177.5 35.5l384-256z";
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
    exports.faBaguette = exports.definition;
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

// src/_entries/faBaguette.js
var import_faBaguette = __toESM(require_faBaguette());
var export_faBaguette = import_faBaguette.definition;
export {
  export_faBaguette as faBaguette
};
