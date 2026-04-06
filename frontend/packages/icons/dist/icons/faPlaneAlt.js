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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPlaneEngines.js
var require_faPlaneEngines = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPlaneEngines.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "plane-engines";
    var width = 576;
    var height = 512;
    var aliases = [9992, "plane-alt"];
    var unicode = "f3de";
    var svgPathData = "M576 256c0-35-59.5-64-93.7-64l-116.6 0-37.2-65.1C342 123.1 352 110.7 352 96c0-17.7-14.3-32-32-32l-27.4 0L265.2 16.1C259.5 6.2 248.9 0 237.4 0L181.2 0c-10.6 0-18.3 10.2-15.4 20.4l49 171.6L112 192 68.8 134.4c-3-4-7.8-6.4-12.8-6.4l-42 0c-7.8 0-14 6.3-14 14c0 1.3 .2 2.6 .5 3.9l22.7 79.3C9.8 229 0 241.4 0 256s9.8 27 23.2 30.8L.5 366.1c-.4 1.3-.5 2.6-.5 3.9c0 7.8 6.3 14 14 14l42 0c5 0 9.8-2.4 12.8-6.4L112 320l102.9 0-49 171.6c-2.9 10.2 4.8 20.4 15.4 20.4l56.2 0c11.5 0 22.1-6.2 27.8-16.1L292.6 448l27.4 0c17.7 0 32-14.3 32-32c0-14.7-10-27.1-23.5-30.9L365.7 320l116.6 0c34.2 0 93.7-28 93.7-64z";
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
    exports.faPlaneEngines = exports.definition;
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

// src/_entries/faPlaneAlt.js
var import_faPlaneEngines = __toESM(require_faPlaneEngines());
var export_faPlaneAlt = import_faPlaneEngines.definition;
export {
  export_faPlaneAlt as faPlaneAlt
};
