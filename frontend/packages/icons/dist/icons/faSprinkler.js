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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSprinkler.js
var require_faSprinkler = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSprinkler.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sprinkler";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e035";
    var svgPathData = "M24 0a24 24 0 1 1 0 48A24 24 0 1 1 24 0zM0 128a24 24 0 1 1 48 0A24 24 0 1 1 0 128zM0 232a24 24 0 1 1 48 0A24 24 0 1 1 0 232zM64 72a24 24 0 1 1 48 0A24 24 0 1 1 64 72zm24 88a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm40-32a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm360 80a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm24-80a24 24 0 1 1 -48 0 24 24 0 1 1 48 0zM488 0a24 24 0 1 1 0 48 24 24 0 1 1 0-48zM448 72a24 24 0 1 1 -48 0 24 24 0 1 1 48 0zm-24 88a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm-40-32a24 24 0 1 1 -48 0 24 24 0 1 1 48 0zM256 64c17.7 0 32 14.3 32 32l0 160 100.2 0c15.3 0 27.8 12.4 27.8 27.8c0 7.8-3.3 15.2-9 20.5L320 384l0 96c0 17.7-14.3 32-32 32l-64 0c-17.7 0-32-14.3-32-32l0-96-87-79.7c-5.7-5.3-9-12.7-9-20.5c0-15.3 12.4-27.8 27.8-27.8L224 256l0-160c0-17.7 14.3-32 32-32z";
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
    exports.faSprinkler = exports.definition;
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

// src/_entries/faSprinkler.js
var import_faSprinkler = __toESM(require_faSprinkler());
var export_faSprinkler = import_faSprinkler.definition;
export {
  export_faSprinkler as faSprinkler
};
