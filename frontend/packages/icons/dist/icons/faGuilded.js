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

// src/node_modules/@fortawesome/free-brands-svg-icons/faGuilded.js
var require_faGuilded = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faGuilded.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "guilded";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e07e";
    var svgPathData = "M443.9 64L5 64c0 103.3 22.2 180.1 43.4 222.4 64.1 127.8 176 161.6 177.3 161.6 55.7-20.5 104.5-56.3 140.6-103.5 25.9-33.9 53.1-87.2 65.9-145.8l-259.9 0c4.1 36.4 22.2 67.9 45.1 86.9l88.6 0c-17 28.2-48.2 54.4-80.5 69.5-31.2-13.3-69.1-46.5-96.5-98.4-26.7-53.8-27.1-105.9-27.1-105.9l336.1 0c4-28.8 5.9-57.9 5.9-86.9z";
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
    exports.faGuilded = exports.definition;
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

// src/_entries/faGuilded.js
var import_faGuilded = __toESM(require_faGuilded());
var export_faGuilded = import_faGuilded.definition;
export {
  export_faGuilded as faGuilded
};
