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

// src/node_modules/@fortawesome/free-solid-svg-icons/faParachuteBox.js
var require_faParachuteBox = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faParachuteBox.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "parachute-box";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f4cd";
    var svgPathData = "M128 224C128 152.2 144.3 88.4 169.5 43.4 195.1-2.4 226.6-24 256-24s60.9 21.6 86.5 67.4C367.7 88.4 384 152.2 384 224l-104 0 0 96 8 0c9.3 0 18.1 2 26 5.5l118.4-101.5-.5 0c0-78.4-17.7-150.6-47.6-204-6.1-11-12.9-21.3-20.3-30.9 86.5 35.9 147.8 115.1 147.8 218.9 0 7-3.1 13.7-8.4 18.2L347.4 360.1c3 7.4 4.6 15.4 4.6 23.9l0 64c0 35.3-28.7 64-64 64l-64 0c-35.3 0-64-28.7-64-64l0-64c0-8.4 1.6-16.5 4.6-23.9L8.4 226.2C3.1 221.7 0 215 0 208 0 104.2 61.3 24.9 147.8-10.9 140.5-1.4 133.7 9 127.6 20 97.7 73.4 80 145.6 80 224l-.5 0 118.4 101.5c8-3.5 16.8-5.5 26-5.5l8 0 0-96-104 0z";
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
    exports.faParachuteBox = exports.definition;
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

// src/_entries/faParachuteBox.js
var import_faParachuteBox = __toESM(require_faParachuteBox());
var export_faParachuteBox = import_faParachuteBox.definition;
export {
  export_faParachuteBox as faParachuteBox
};
