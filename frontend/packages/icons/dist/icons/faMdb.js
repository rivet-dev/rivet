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

// src/node_modules/@fortawesome/free-brands-svg-icons/faMdb.js
var require_faMdb = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faMdb.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "mdb";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f8ca";
    var svgPathData = "M17.4 160.4l-10.4 191.6 43.9 0 5.6-79.8 27.9 79.8 44.7 0 25.5-77.4 4.8 77.4 45.5 0-12.8-191.6-45.5 0-40.7 117.3-42.3-117.3-46.3 0zm281 0l-47.9 0 0 191.6 47.9 0s95 .8 94.2-95.8c-.8-94.2-94.2-95.8-94.2-95.8l0 0zm-1.2 146.5l0-102.1s46 4.3 46.8 50.6-46.8 51.5-46.8 51.5l0 0zm238.3-74.2c7.2-11.4 10-25 8-38.3-5.3-35.8-55.1-34.3-55.1-34.3l-51.9 0 0 191.6 45.5 0s87 4.8 87-63.8c0-43.1-33.5-55.1-33.5-55.1l0 0zm-51.9-31.9s13.6-1.6 16 9.6c1.4 6.7-4 12-4 12l-12 0 0-21.6zm-.1 109.5l.1-24.9 0-18.2 .1 0s41.6-4.7 41.2 22.4c-.3 25.7-41.3 20.7-41.3 20.7l0 0z";
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
    exports.faMdb = exports.definition;
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

// src/_entries/faMdb.js
var import_faMdb = __toESM(require_faMdb());
var export_faMdb = import_faMdb.definition;
export {
  export_faMdb as faMdb
};
