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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSportsball.js
var require_faSportsball = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSportsball.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sportsball";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e44b";
    var svgPathData = "M487.6 48.9c-.1-.5-.2-1-.3-1.5L365.3 169.3c34.8 30.2 79 49.8 127.6 53.9c9.3-63 4.7-125.5-5.4-174.3zM464.8 24.6c-1.1-.3-2.2-.5-3.3-.7c-65.6-11-122.9-12.5-172.8-6c3.9 49.1 23.6 93.7 54 128.8L464.8 24.6zM446 358.4c19.8-32.3 33-67.7 41.1-103.6c-55.1-5.3-105.1-28.1-144.4-62.8l-64 64L418.5 395.9c10.3-11.8 19.4-24.4 27.4-37.5zM320 169.4c-35-39.7-57.9-90.3-63-146.1c-17 3.6-33 8.3-48.1 13.9c-37 13.7-67.8 33-93.1 56L256 233.4l64-64zM233.4 256L93.4 116.1C83.2 127.9 74.1 140.5 66 153.6c-19.8 32.3-33 67.7-41.1 103.6c55.1 5.3 105.1 28.1 144.4 62.8l64-64zM192 342.6c35 39.7 58 90.3 63 146.1c17-3.6 33-8.3 48.1-13.9c37-13.7 67.8-33 93.1-56L256 278.6l-64 64zm-45.3 .1c-34.8-30.2-79-49.8-127.6-53.9c-9.3 63-4.7 125.5 5.4 174.3c.1 .5 .2 1 .3 1.5L146.7 342.7zm22.6 22.6L47.2 487.4c1.1 .3 2.2 .5 3.3 .7c65.6 11 122.9 12.5 172.8 6c-3.9-49.1-23.6-93.7-54-128.8z";
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
    exports.faSportsball = exports.definition;
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

// src/_entries/faSportsball.js
var import_faSportsball = __toESM(require_faSportsball());
var export_faSportsball = import_faSportsball.definition;
export {
  export_faSportsball as faSportsball
};
