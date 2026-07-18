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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAnt.js
var require_faAnt = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAnt.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ant";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e680";
    var svgPathData = "M523.3 51.9c1.4-2.4 4.1-3.9 6.9-3.9L584 48c13.3 0 24-10.7 24-24s-10.7-24-24-24L530.1 0c-19.7 0-37.9 10.3-48 27.2L458.6 66.4C415.9 76.1 384 114.3 384 160c0 14.8 3.3 28.8 9.3 41.3c-12.5-6-26.5-9.3-41.3-9.3c-27 0-51.4 11.2-68.9 29.1C267.9 167.6 218.6 128 160.4 128C72 128 0 199.6 0 288c0 44.2 35.8 80 80 80l48.9 0L67.2 476.1c-6.6 11.5-2.6 26.2 8.9 32.7s26.2 2.6 32.7-8.9l75.6-132.2c9.7-.7 19.1-2.7 28-5.7L147.2 476.1c-6.6 11.5-2.6 26.2 8.9 32.7s26.2 2.6 32.7-8.9L276.2 347c4.6 5.9 9.8 11.2 15.7 15.9l-.5 .8-.3 .4-64 112c-6.6 11.5-2.6 26.2 8.9 32.7s26.2 2.6 32.7-8.9l63.9-111.8 3.3-5.5c5.2 .9 10.6 1.3 16 1.3c13.6 0 26.5-2.8 38.2-7.9l39.6 39.6c3.8 3.8 6.8 8.4 8.9 13.4l27.1 67.8c4.9 12.3 18.9 18.3 31.2 13.4s18.3-18.9 13.4-31.2l-27.1-67.8c-4.4-11.1-11.1-21.1-19.5-29.5l-35.4-35.4c2.2-2.8 4.2-5.8 6.1-8.9l86.8 62c6.9 4.9 12 11.9 14.7 19.9l25.4 76.3c4.2 12.6 17.8 19.4 30.4 15.2s19.4-17.8 15.2-30.4l-25.4-76.3c-5.9-17.6-17.2-33-32.3-43.8l-95.1-67.9-5.9-4.2 0-.2c0-14.8-3.3-28.8-9.3-41.3c12.5 6 26.5 9.3 41.3 9.3l64 0c35.3 0 64-28.7 64-64c0-59.2-40.1-108.9-94.6-123.6l9.9-16.5zM512 136a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
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
    exports.faAnt = exports.definition;
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

// src/_entries/faAnt.js
var import_faAnt = __toESM(require_faAnt());
var export_faAnt = import_faAnt.definition;
export {
  export_faAnt as faAnt
};
