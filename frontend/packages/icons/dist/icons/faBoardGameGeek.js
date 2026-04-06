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

// src/node_modules/@fortawesome/free-brands-svg-icons/faBoardGameGeek.js
var require_faBoardGameGeek = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faBoardGameGeek.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "board-game-geek";
    var width = 640;
    var height = 512;
    var aliases = ["bgg"];
    var unicode = "e855";
    var svgPathData = "M168.8 168.3l30.1-8.2-12.3 98.8 16.7 16.1-38.9 91.2-111.8 41.2-52.6-131.8 17.7-17-10.5-84.3 191.2-70.3-29.5 64.3zm180.6 16.5c11.6 0 19 7.4 19 17.9l0 31.5c0 9.7-6.3 18.9-19.5 22.1 13.1 3.1 20.5 12 20.5 21.4l0 30.9c0 11.1-7.4 18.1-19 18.1l-89.1 0 0-141.8 88 0zM298 299.8l33 0c1.5 0 2.1-1 2.1-2.1l0-26.9c0-1-.6-2.3-2.1-2.3l-33 0 0 31.3zm0-57.3l33 0c1.5 0 2.1-1.1 2.1-2.3l0-24.8c0-1-.6-2.3-2.1-2.3l-33 0 0 29.4zm204.1-27.2l-65.6 0c-1 0-2.1 1.1-2.1 2.3l0 76.1c0 1.3 1 2.1 2.1 2.1l29.7 0c1.5 0 2.3-.8 2.3-2.1l0-24-19.3 0 12.8-30 40.1 0 0 70.8c0 9.2-7.2 16.2-16.3 16.2l-71.5 0c-9.1 0-16.5-7-16.5-16.2l0-109.7c0-8.6 7.4-16 16.5-16l87.8 0 0 30.5zm132.6 0l-65.6 0c-1.1 0-2.1 1.1-2.1 2.3l0 76.1c0 1.3 1 2.1 2.1 2.1l29.6 0c1.5 0 2.3-.8 2.3-2.1l0-24-19.2 0 12.8-30 40.1 0 0 70.8c0 9.2-7.2 16.2-16.3 16.2l-71.5 0c-9.1 0-16.5-6.9-16.5-16.2l0-109.7c0-8.6 7.4-16 16.5-16l87.8 0 0 30.5z";
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
    exports.faBoardGameGeek = exports.definition;
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

// src/_entries/faBoardGameGeek.js
var import_faBoardGameGeek = __toESM(require_faBoardGameGeek());
var export_faBoardGameGeek = import_faBoardGameGeek.definition;
export {
  export_faBoardGameGeek as faBoardGameGeek
};
