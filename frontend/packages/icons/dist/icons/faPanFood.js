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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPanFood.js
var require_faPanFood = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPanFood.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pan-food";
    var width = 640;
    var height = 512;
    var aliases = [129368];
    var unicode = "e42b";
    var svgPathData = "M320 512A256 256 0 1 0 320 0a256 256 0 1 0 0 512zm0-448c37.2 0 69.3 26.3 76.6 62.8l.6 3.2c1.4 7.3-4.1 14-11.5 14l-131.4 0c-7.4 0-12.9-6.8-11.5-14l.6-3.2C250.7 90.3 282.8 64 320 64zM230.1 185.9c7.8 7.8 7.8 20.5 0 28.3l-32 32c-7.8 7.8-20.5 7.8-28.3 0s-7.8-20.5 0-28.3l32-32c7.8-7.8 20.5-7.8 28.3 0zM296 224a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zM416 360a24 24 0 1 1 0 48 24 24 0 1 1 0-48zM227.3 284.7c2.1 2.1 3.5 4.6 4.2 7.3c.4 1.7 .8 3.4 1.1 5.1c1.1 5.4 2.1 11 5.9 14.7l3.4 3.4c3 3 7.1 4.7 11.3 4.7l26.7 0c30.9 0 56 25.1 56 56s-25.1 56-56 56s-56-25.1-56-56l0-26.7c0-4.2-1.7-8.3-4.7-11.3l-3.4-3.4c-3.8-3.8-9.3-4.8-14.7-5.9c-1.7-.3-3.5-.7-5.1-1.1c-2.7-.7-5.2-2.1-7.3-4.2c-6.2-6.2-6.2-16.4 0-22.6l8-8 8-8c6.2-6.2 16.4-6.2 22.6 0zM455.8 184.2c4.5-4.5 11.9-4.5 16.4 0l2.8 2.8c13.4 13.4 21 31.6 21 50.6l0 4.8c0 18.7-8.5 36.4-23.1 48.1C462 299.2 448.4 304 434.4 304l-4.1 0c-19.4 0-38-7.7-51.8-21.4l-2.4-2.4c-4.5-4.5-4.5-11.9 0-16.4l79.6-79.6zM24 160c-13.3 0-24 10.7-24 24L0 328c0 13.3 10.7 24 24 24s24-10.7 24-24l0-144c0-13.3-10.7-24-24-24zm616 24c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 144c0 13.3 10.7 24 24 24s24-10.7 24-24l0-144z";
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
    exports.faPanFood = exports.definition;
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

// src/_entries/faPanFood.js
var import_faPanFood = __toESM(require_faPanFood());
var export_faPanFood = import_faPanFood.definition;
export {
  export_faPanFood as faPanFood
};
