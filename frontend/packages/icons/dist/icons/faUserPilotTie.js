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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserPilotTie.js
var require_faUserPilotTie = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserPilotTie.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-pilot-tie";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e2c1";
    var svgPathData = "M230.1 .8c-4-1.1-8.2-1.1-12.2 0l-152 40c-8.6 2.3-15.3 9.1-17.3 17.8s1 17.8 7.8 23.6L80 102.5l0 8.4c0 10.7 5.3 20.8 15.1 25.2c24.1 10.8 68.6 24 128.9 24s104.8-13.2 128.9-24c9.8-4.4 15.1-14.5 15.1-25.2l0-8.4 23.6-20.2c6.8-5.8 9.8-14.9 7.8-23.6s-8.7-15.6-17.3-17.8l-152-40zM161.7 72.8c4-7.9 13.6-11.1 21.5-7.2L224 86.1l40.8-20.4c7.9-4 17.5-.7 21.5 7.2s.7 17.5-7.2 21.5l-48 24c-4.5 2.3-9.8 2.3-14.3 0l-48-24c-7.9-4-11.1-13.6-7.2-21.5zm-63.6 96C96.7 176.3 96 184.1 96 192c0 70.7 57.3 128 128 128s128-57.3 128-128c0-7.9-.7-15.7-2.1-23.2C310.8 183.8 268.4 192 224 192s-86.8-8.2-125.9-23.2zm89.5 206.3L208 416l-12.2 48.9L133 360.3c-3-5-8.6-8.1-14.4-7.4C51.8 360.8 0 417.5 0 486.4C0 500.5 11.5 512 25.6 512L192 512s0 0 0 0l64 0s0 0 0 0l166.4 0c14.1 0 25.6-11.5 25.6-25.6c0-68.9-51.8-125.6-118.6-133.5c-5.8-.7-11.4 2.4-14.4 7.4L252.2 464.9 240 416l20.4-40.8c5.3-10.6-2.4-23.2-14.3-23.2l-44.2 0c-11.9 0-19.6 12.5-14.3 23.2zM320 432c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16z";
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
    exports.faUserPilotTie = exports.definition;
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

// src/_entries/faUserPilotTie.js
var import_faUserPilotTie = __toESM(require_faUserPilotTie());
var export_faUserPilotTie = import_faUserPilotTie.definition;
export {
  export_faUserPilotTie as faUserPilotTie
};
