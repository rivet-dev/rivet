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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHockeySticks.js
var require_faHockeySticks = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHockeySticks.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hockey-sticks";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f454";
    var svgPathData = "M220.6 17.7l82.3 164.6-35.8 71.6L163.4 46.3c-7.9-15.8-1.5-35 14.3-42.9s35-1.5 42.9 14.3zM372.8 322.1L387.8 352 544 352l0 128 0 32-64 0s0 0 0 0l-44.2 0c-24.2 0-46.4-13.7-57.2-35.4l-41.5-83 35.8-71.6zM576 352l32 0c17.7 0 32 14.3 32 32l0 96c0 17.7-14.3 32-32 32l-32 0 0-160zM462.3 3.4c15.8 7.9 22.2 27.1 14.3 42.9L261.5 476.6c-10.8 21.7-33 35.4-57.2 35.4L160 512l-32 0-32 0 0-32 0-128 156.2 0L419.4 17.7c7.9-15.8 27.1-22.2 42.9-14.3zM0 384c0-17.7 14.3-32 32-32l32 0 0 160-32 0c-17.7 0-32-14.3-32-32l0-96z";
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
    exports.faHockeySticks = exports.definition;
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

// src/_entries/faHockeySticks.js
var import_faHockeySticks = __toESM(require_faHockeySticks());
var export_faHockeySticks = import_faHockeySticks.definition;
export {
  export_faHockeySticks as faHockeySticks
};
