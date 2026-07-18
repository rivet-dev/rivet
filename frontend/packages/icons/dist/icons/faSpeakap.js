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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSpeakap.js
var require_faSpeakap = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSpeakap.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "speakap";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f3f3";
    var svgPathData = "M64.4 391.8C-15 303.6-7.6 167.4 81.1 87.6s224.8-73 304.2 15.2 72 224.4-16.6 304.1c-18.7 16.9 64 43.1 42 52.3-82.1 34.2-253.9 35-346.2-67.5l0 0zM277.7 180.2l38.5-40.9c-9.6-8.9-32-26.8-76.2-27.6-52.3-.9-95.9 28.3-96.8 80-.2 11.3 .3 36.7 29.4 54.8 34.5 21.4 86.5 21.5 86 52.3-.4 21.3-26.4 25.8-38.6 25.6-3 0-30.2-.5-47.6-24.6l-40 42.6c28.2 27 59 32.6 83.5 33 10.2 .2 96.4 .3 97.8-81 .3-15.8-2.1-39.7-28.9-56.6-34.4-21.6-85-19.4-84.4-49.7 .4-23.3 31-25.4 37.5-25.3 .4 0 26.6 .3 39.6 17.4l0 0z";
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
    exports.faSpeakap = exports.definition;
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

// src/_entries/faSpeakap.js
var import_faSpeakap = __toESM(require_faSpeakap());
var export_faSpeakap = import_faSpeakap.definition;
export {
  export_faSpeakap as faSpeakap
};
