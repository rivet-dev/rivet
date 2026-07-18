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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonThroughWindow.js
var require_faPersonThroughWindow = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonThroughWindow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-through-window";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e5a9";
    var svgPathData = "M64 64l224 0 0 9.8c0 39-23.7 74-59.9 88.4-60.5 24.2-100.1 82.7-100.1 147.9l0 73.8-64 0 0-320zm288 0l224 0 0 320-67.7 0-3.7-4.5-75.2-90.2c-9.1-10.9-22.6-17.3-36.9-17.3l-71.1 0-41-63.1c-.3-.5-.6-1-1-1.4 44.7-29 72.5-79 72.5-133.6l0-9.8zm73 320l-26.6 0 42.7 64 150.9 0c26.5 0 48-21.5 48-48l0-352c0-26.5-21.5-48-48-48L48 0C21.5 0 0 21.5 0 48L0 400c0 26.5 21.5 48 48 48l260.2 0 33.2 49.8c9.8 14.7 29.7 18.7 44.4 8.9s18.7-29.7 8.9-44.4l-84.2-126.2 74.6 0 40 48zm-159.5 0l-73.5 0 0-73.8c0-10.2 1.6-20.1 4.7-29.5L265.5 384zM192 128a48 48 0 1 0 -96 0 48 48 0 1 0 96 0z";
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
    exports.faPersonThroughWindow = exports.definition;
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

// src/_entries/faPersonThroughWindow.js
var import_faPersonThroughWindow = __toESM(require_faPersonThroughWindow());
var export_faPersonThroughWindow = import_faPersonThroughWindow.definition;
export {
  export_faPersonThroughWindow as faPersonThroughWindow
};
