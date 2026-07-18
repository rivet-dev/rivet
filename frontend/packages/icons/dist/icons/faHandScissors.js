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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHandScissors.js
var require_faHandScissors = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHandScissors.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hand-scissors";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f257";
    var svgPathData = "M40 208c-22.1 0-40 17.9-40 40s17.9 40 40 40l180.2 0c-7.6 8.5-12.2 19.7-12.2 32 0 25.3 19.5 46 44.3 47.9-7.7 8.5-12.3 19.8-12.3 32.1 0 26.5 21.5 48 48 48l96 0c70.7 0 128-57.3 128-128l0-113.1c0-40.2-16-78.8-44.4-107.3-22.8-22.8-53.6-35.6-85.8-35.6L336 64c-21.3 0-39.3 13.9-45.6 33.1l74.5 23.7c8.4 2.7 13.1 11.7 10.4 20.1s-11.7 13.1-20.1 10.4l-67.1-21.4 0 .1-204-64.1C62.9 59.2 40.5 70.9 33.9 92s5.1 43.5 26.2 50.2L269.5 208 40 208z";
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
    exports.faHandScissors = exports.definition;
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

// src/_entries/faHandScissors.js
var import_faHandScissors = __toESM(require_faHandScissors());
var export_faHandScissors = import_faHandScissors.definition;
export {
  export_faHandScissors as faHandScissors
};
