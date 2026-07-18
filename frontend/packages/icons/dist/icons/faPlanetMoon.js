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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPlanetMoon.js
var require_faPlanetMoon = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPlanetMoon.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "planet-moon";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e01f";
    var svgPathData = "M512 0a64 64 0 1 1 0 128A64 64 0 1 1 512 0zM0 288a224 224 0 1 1 448 0A224 224 0 1 1 0 288zM236.2 112.4c-4-.3-8.1-.4-12.2-.4c-76.2 0-141.1 48.4-165.6 116.2C53.3 242.3 64.7 256 79.7 256l28.4 0c12.7 0 24.9 5.1 33.9 14.1L185 313c4.5 4.5 7 10.6 7 17l0 22.1c0 9.2 3.9 17.6 10.2 23.4l1.9 1.8c9.3 8.6 19.9 18.4 19.9 30.8l0 23c0 13.8 11.2 25 25 25c9.2 0 17.7-5.1 22-13.2l41.3-76.6c5-9.3 7.6-19.8 7.6-30.3l0-15.9c0-17.7-14.3-32-32-32l-65.9 0c-9 0-17.7-3.6-24.1-10c-12.5-12.5-13.4-32.4-2.1-45.9l52.7-63.2c4.8-5.8 7.4-13 7.4-20.5l0-13.5c0-11.6-8.3-21.7-19.8-22.5z";
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
    exports.faPlanetMoon = exports.definition;
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

// src/_entries/faPlanetMoon.js
var import_faPlanetMoon = __toESM(require_faPlanetMoon());
var export_faPlanetMoon = import_faPlanetMoon.definition;
export {
  export_faPlanetMoon as faPlanetMoon
};
