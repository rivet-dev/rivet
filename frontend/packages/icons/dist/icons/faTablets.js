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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTablets.js
var require_faTablets = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTablets.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tablets";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f490";
    var svgPathData = "M495.3 76.6c-4.6-8.7-16.3-9.7-23.3-2.7L297.9 248c-7 7-6 18.7 2.7 23.3 20.1 10.7 43 16.7 67.4 16.7 79.5 0 144-64.5 144-144 0-24.3-6-47.3-16.7-67.4zM240.7 211.4c4.6 8.7 16.3 9.7 23.3 2.7L438.1 40c7-7 6-18.7-2.7-23.3-20.1-10.7-43-16.7-67.4-16.7-79.5 0-144 64.5-144 144 0 24.3 6 47.3 16.7 67.4zM144 224C79.2 224 24.4 266.8 6.3 325.6 3.4 335.1 11 344 20.9 344l246.2 0c9.9 0 17.5-8.9 14.6-18.4-.9-3-1.9-5.9-3-8.7-37.4-18.1-68-48-87-84.8-14.9-5.2-30.9-8.1-47.6-8.1zM281.7 410.4c2.9-9.5-4.7-18.4-14.6-18.4L20.9 392C11 392 3.4 400.9 6.3 410.4 24.4 469.2 79.2 512 144 512s119.6-42.8 137.7-101.6z";
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
    exports.faTablets = exports.definition;
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

// src/_entries/faTablets.js
var import_faTablets = __toESM(require_faTablets());
var export_faTablets = import_faTablets.definition;
export {
  export_faTablets as faTablets
};
