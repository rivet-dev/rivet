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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTicketAirline.js
var require_faTicketAirline = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTicketAirline.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ticket-airline";
    var width = 640;
    var height = 512;
    var aliases = ["ticket-perforated-plane", "ticket-plane"];
    var unicode = "e29a";
    var svgPathData = "M64 64C28.7 64 0 92.7 0 128L0 384c0 35.3 28.7 64 64 64l512 0c35.3 0 64-28.7 64-64l0-48c0-8.8-7.3-15.8-15.8-18c-27.7-7-48.2-32.1-48.2-62s20.5-55 48.2-62c8.6-2.2 15.8-9.1 15.8-18l0-48c0-35.3-28.7-64-64-64L64 64zm352 64a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zm0 64a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zm16 48a16 16 0 1 1 0 32 16 16 0 1 1 0-32zm-16 80a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zm16 48a16 16 0 1 1 0 32 16 16 0 1 1 0-32zM176 224l-28.5-85.5c-1.7-5.2 2.1-10.5 7.6-10.5l31.1 0c8.5 0 16.4 4.5 20.7 11.9L256 224l46.9 0c16.1 0 31.5 6.4 42.8 17.7c7.9 7.9 7.9 20.7 0 28.5C334.4 281.6 319 288 302.9 288L256 288l-49.1 84.1c-4.3 7.4-12.2 11.9-20.7 11.9l-31.1 0c-5.5 0-9.3-5.4-7.6-10.5L176 288l-56 0-19.2 25.6c-3 4-7.8 6.4-12.8 6.4l-13.8 0c-5.2 0-9-4.9-7.8-9.9L79 259.9c.6-2.5 .6-5.2 0-7.8L66.5 201.9c-1.3-5 2.6-9.9 7.8-9.9L88 192c5 0 9.8 2.4 12.8 6.4L120 224l56 0z";
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
    exports.faTicketAirline = exports.definition;
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

// src/_entries/faTicketPlane.js
var import_faTicketAirline = __toESM(require_faTicketAirline());
var export_faTicketPlane = import_faTicketAirline.definition;
export {
  export_faTicketPlane as faTicketPlane
};
