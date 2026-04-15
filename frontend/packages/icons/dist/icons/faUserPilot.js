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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserPilot.js
var require_faUserPilot = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserPilot.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-pilot";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e2c0";
    var svgPathData = "M230.1 .8c-4-1.1-8.2-1.1-12.2 0l-152 40c-8.6 2.3-15.3 9.1-17.3 17.8s1 17.8 7.8 23.6L80 102.5l0 8.4c0 10.7 5.3 20.8 15.1 25.2c24.1 10.8 68.6 24 128.9 24s104.8-13.2 128.9-24c9.8-4.4 15.1-14.5 15.1-25.2l0-8.4 23.6-20.2c6.8-5.8 9.8-14.9 7.8-23.6s-8.7-15.6-17.3-17.8l-152-40zM161.7 72.8c4-7.9 13.6-11.1 21.5-7.2L224 86.1l40.8-20.4c7.9-4 17.5-.7 21.5 7.2s.7 17.5-7.2 21.5l-48 24c-4.5 2.3-9.8 2.3-14.3 0l-48-24c-7.9-4-11.1-13.6-7.2-21.5zm-63.6 96C96.7 176.3 96 184.1 96 192c0 70.7 57.3 128 128 128s128-57.3 128-128c0-7.9-.7-15.7-2.1-23.2C310.8 183.8 268.4 192 224 192s-86.8-8.2-125.9-23.2zm116.3 240l-72.3-54.2c-2.2-1.7-4.9-2.6-7.7-2.6C60.2 352 0 412.2 0 486.4C0 500.5 11.5 512 25.6 512l396.8 0c14.1 0 25.6-11.5 25.6-25.6C448 412.2 387.8 352 313.6 352c-2.8 0-5.5 .9-7.7 2.6l-72.3 54.2c-5.7 4.3-13.5 4.3-19.2 0zM336 416l32 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16s7.2-16 16-16z";
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
    exports.faUserPilot = exports.definition;
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

// src/_entries/faUserPilot.js
var import_faUserPilot = __toESM(require_faUserPilot());
var export_faUserPilot = import_faUserPilot.definition;
export {
  export_faUserPilot as faUserPilot
};
