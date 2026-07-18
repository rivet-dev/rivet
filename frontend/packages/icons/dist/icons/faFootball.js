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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFootball.js
var require_faFootball = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFootball.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "football";
    var width = 512;
    var height = 512;
    var aliases = [127944, "football-ball"];
    var unicode = "f44e";
    var svgPathData = "M261.1 22.6c-89 18-150.5 63.4-190 123.9-23.3 35.6-38.1 75.3-46.7 115.5L251.9 489.4c89-18 150.5-63.4 190.1-123.9 23.3-35.6 38.1-75.3 46.7-115.5L261.1 22.6zm236 168.1c3.2-42.3 .7-83.3-4.8-118.7-4.4-27.8-26.8-48-53.1-51.6-43-5.9-82.2-7.5-117.8-5.4L497.1 190.6zM191.7 497.1L15.9 321.4c-3.2 42.3-.7 83.3 4.8 118.7 4.4 27.8 26.8 48 53.1 51.6 43 5.9 82.2 7.5 117.8 5.4zM271.5 143c9.4-9.4 24.6-9.4 33.9 0l64 64c9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0l-64-64c-9.4-9.4-9.4-24.6 0-33.9zm-64 64c9.4-9.4 24.6-9.4 33.9 0l64 64c9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0l-64-64c-9.4-9.4-9.4-24.6 0-33.9zm-64 64c9.4-9.4 24.6-9.4 33.9 0l64 64c9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0l-64-64c-9.4-9.4-9.4-24.6 0-33.9z";
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
    exports.faFootball = exports.definition;
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

// src/_entries/faFootball.js
var import_faFootball = __toESM(require_faFootball());
var export_faFootball = import_faFootball.definition;
export {
  export_faFootball as faFootball
};
