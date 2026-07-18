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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHundredPoints.js
var require_faHundredPoints = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHundredPoints.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hundred-points";
    var width = 512;
    var height = 512;
    var aliases = [128175, "100"];
    var unicode = "e41c";
    var svgPathData = "M163 101.8C168 61.9 201.9 32 242 32c47.9 0 85 42 79 89.5L309 218.2c-5 39.8-38.9 69.8-79 69.8c-47.9 0-85-42-79-89.5L163 101.8zM242 96c-7.9 0-14.5 5.9-15.5 13.7l-12.1 96.7c-1.2 9.3 6.1 17.6 15.5 17.6c7.9 0 14.5-5.9 15.5-13.7l12.1-96.7c1.2-9.3-6.1-17.6-15.5-17.6zM135.7 68l-32 224c-2.5 17.5-18.7 30-36.2 27.7S37.8 301.5 40.3 284L64.3 116.5l-19.7 8.7c-16.2 7.2-35.1 0-42-16.1s.6-35.1 16.8-42.3l72-32c10.6-4.7 22.9-3.4 32.1 3.5s13.8 18.2 12.2 29.7zm376 247.5c2.5 17.5-9.7 33.7-27.2 36.2l-448 64C19 418.2 2.8 406 .3 388.5S10 354.8 27.5 352.3l448-64c17.5-2.5 33.7 9.7 36.2 27.2zm0 96c2.5 17.5-9.7 33.7-27.2 36.2l-224 32c-17.5 2.5-33.7-9.7-36.2-27.2s9.7-33.7 27.2-36.2l224-32c17.5-2.5 33.7 9.7 36.2 27.2zM427.7 32c46.9 0 83.7 40.3 79.5 87l-7.3 80.4c-3.7 41.1-38.2 72.6-79.5 72.6c-46.9 0-83.7-40.3-79.5-87l7.3-80.4C351.9 63.5 386.4 32 427.7 32zm-15.7 78.4l-7.3 80.4c-.8 9.3 6.4 17.2 15.7 17.2c8.2 0 15-6.2 15.7-14.4l7.3-80.4c.8-9.3-6.4-17.2-15.7-17.2c-8.2 0-15 6.2-15.7 14.4z";
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
    exports.faHundredPoints = exports.definition;
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

// src/_entries/fa100.js
var import_faHundredPoints = __toESM(require_faHundredPoints());
var export_fa100 = import_faHundredPoints.definition;
export {
  export_fa100 as fa100
};
