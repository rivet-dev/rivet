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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBroomBall.js
var require_faBroomBall = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBroomBall.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "broom-ball";
    var width = 576;
    var height = 512;
    var aliases = ["quidditch", "quidditch-broom-ball"];
    var unicode = "f458";
    var svgPathData = "M496 544a80 80 0 1 1 0-160 80 80 0 1 1 0 160zM106.8 277.2c30.6-30.6 73.1-45.9 115.8-42.2L341 353.3c3.7 42.8-11.6 85.2-42.2 115.9-27.4 27.4-64.6 42.8-103.3 42.8L22.1 512c-12.2 0-22.1-9.9-22.1-22.1 0-6.3 2.7-12.3 7.3-16.5L133.7 359.7c4.2-3.7-.4-10.4-5.4-7.9L77.2 377.4c-6.1 3-13.2-1.4-13.2-8.2 0-31.5 12.5-61.8 34.8-84l8-8zm417-270c12.6-10.3 31.1-9.5 42.8 2.2s12.4 30.2 2.2 42.8l-2.2 2.4-192 192 34.8 34.7c4.2 4.2 6.6 10 6.6 16 0 12.5-10.1 22.6-22.6 22.6l-29.1 0-108.3-108.3 0-29.1c0-12.5 10.1-22.6 22.6-22.6 6 0 11.8 2.4 16 6.6l34.8 34.7 192-192 2.4-2.2z";
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
    exports.faBroomBall = exports.definition;
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

// src/_entries/faQuidditch.js
var import_faBroomBall = __toESM(require_faBroomBall());
var export_faQuidditch = import_faBroomBall.definition;
export {
  export_faQuidditch as faQuidditch
};
