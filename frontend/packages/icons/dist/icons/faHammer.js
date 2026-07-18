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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHammer.js
var require_faHammer = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHammer.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hammer";
    var width = 640;
    var height = 512;
    var aliases = [128296];
    var unicode = "f6e3";
    var svgPathData = "M246.9 18.3L271 3.8c21.6-13 46.3-19.8 71.5-19.8 36.8 0 72.2 14.6 98.2 40.7l63.9 63.9c15 15 23.4 35.4 23.4 56.6l0 30.9 19.7 19.7 0 0c15.6-15.6 40.9-15.6 56.6 0s15.6 40.9 0 56.6l-64 64c-15.6 15.6-40.9 15.6-56.6 0s-15.6-40.9 0-56.6L464 240 433.1 240c-21.2 0-41.6-8.4-56.6-23.4l-49.1-49.1c-15-15-23.4-35.4-23.4-56.6l0-12.7c0-11.2-5.9-21.7-15.5-27.4l-41.6-25c-10.4-6.2-10.4-21.2 0-27.4zM50.7 402.7l222.1-222.1 90.5 90.5-222.1 222.1c-25 25-65.5 25-90.5 0s-25-65.5 0-90.5z";
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
    exports.faHammer = exports.definition;
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

// src/_entries/faHammer.js
var import_faHammer = __toESM(require_faHammer());
var export_faHammer = import_faHammer.definition;
export {
  export_faHammer as faHammer
};
